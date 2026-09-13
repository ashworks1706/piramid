//! Serving the router on a listener until a shutdown signal, then draining and checkpointing.

use std::future::{Future, IntoFuture};
use std::net::SocketAddr;
use std::time::Duration;

use piramid_core::config::{AuthConfig, API_KEY_ENV};
use tokio::net::TcpListener;
use tokio::sync::oneshot;

use super::rate_limit::RateLimit;
use super::routes::create_router;
use crate::state::SharedState;

/// Period on which idle rate limit buckets are dropped.
const RATE_LIMIT_SWEEP: Duration = Duration::from_secs(60);

/// Why the server did not start, or did not stop cleanly.
#[derive(Debug, thiserror::Error)]
pub enum ServeError {
    /// The listener is reachable from other hosts and nothing authenticates requests.
    #[error(
        "refusing to serve {address} without authentication: the address is reachable from other \
         machines and {API_KEY_ENV} is not set. Set {API_KEY_ENV} to the key clients must send, \
         bind a loopback address such as 127.0.0.1:{port} with startup.bind, or set \
         startup.http.auth.allow_unauthenticated: true to serve without a key"
    )]
    Unauthenticated {
        /// The address the listener is bound to.
        address: SocketAddr,
        /// Its port.
        port: u16,
    },
    /// The rate limit settings could not build a limiter.
    #[error("{0}")]
    RateLimit(String),
    /// Accepting or serving connections failed.
    #[error("server: {0}")]
    Io(#[from] std::io::Error),
    /// The checkpoint task did not run to completion.
    #[error("checkpoint at shutdown did not complete: {0}")]
    CheckpointTask(#[source] tokio::task::JoinError),
    /// One or more collections failed to checkpoint at shutdown.
    #[error("checkpoint at shutdown failed for collections: {}", .0.join(", "))]
    Checkpoint(Vec<String>),
    /// The inference shutdown task did not run to completion.
    #[error("inference shutdown task: {0}")]
    InferenceShutdownTask(#[source] tokio::task::JoinError),
}

/// Refuses a non-loopback listener when no key is set and the operator has not opted out.
pub fn check_exposure(address: SocketAddr, auth: &AuthConfig) -> Result<(), ServeError> {
    if address.ip().is_loopback() || auth.api_key.is_some() || auth.allow_unauthenticated {
        return Ok(());
    }
    Err(ServeError::Unauthenticated {
        address,
        port: address.port(),
    })
}

/// Serves until shutdown completes, then drains in-flight requests and checkpoints every open
/// collection.
///
/// Draining waits at most startup.http.drain_timeout_secs. Requests that arrive on the process
/// after the drain are refused as unavailable.
pub async fn serve<F>(
    state: SharedState,
    listener: TcpListener,
    shutdown: F,
) -> Result<(), ServeError>
where
    F: Future<Output = ()> + Send + 'static,
{
    let http = state.http_config().clone();
    let address = listener.local_addr()?;
    check_exposure(address, &http.auth)?;

    let rate_limit = http
        .rate_limit
        .as_ref()
        .map(RateLimit::new)
        .transpose()
        .map_err(ServeError::RateLimit)?;
    let sweeper = rate_limit
        .clone()
        .map(|limit| tokio::spawn(limit.sweep(RATE_LIMIT_SWEEP)));
    let app = create_router(state.clone(), rate_limit.as_ref());

    tracing::info!(
        target: "piramid::http",
        address = %address,
        authenticated = http.auth.api_key.is_some(),
        rate_limited = rate_limit.is_some(),
        "server_listening"
    );

    let (signalled, signal_received) = oneshot::channel::<()>();
    let signal = async move {
        shutdown.await;
        let _ = signalled.send(());
    };
    let server = axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(signal)
    .into_future();
    tokio::pin!(server);

    let served = tokio::select! {
        biased;
        _ = signal_received => {
            tracing::info!(
                target: "piramid::shutdown",
                drain_timeout_secs = http.drain_timeout_secs,
                "shutdown_draining"
            );
            let drain = Duration::from_secs(http.drain_timeout_secs);
            match tokio::time::timeout(drain, &mut server).await {
                Ok(result) => result,
                Err(_) => {
                    tracing::warn!(
                        target: "piramid::shutdown",
                        drain_timeout_secs = http.drain_timeout_secs,
                        "shutdown_drain_timeout_elapsed"
                    );
                    Ok(())
                }
            }
        }
        result = &mut server => result,
    };

    if let Some(sweeper) = sweeper {
        sweeper.abort();
    }
    state.initiate_shutdown();
    let inference_shutdown = match state.inference.clone() {
        Some(manager) => tokio::task::spawn_blocking(move || manager.shutdown()).await,
        None => Ok(()),
    };
    if let Err(error) = &inference_shutdown {
        tracing::error!(
            target: "piramid::shutdown",
            %error,
            "inference_shutdown_task_failed"
        );
    }
    let checkpointed = checkpoint(state).await;
    shutdown_outcome(served, checkpointed, inference_shutdown)
}

/// The first failure of serving, checkpointing and inference shutdown, in that order.
fn shutdown_outcome(
    served: std::io::Result<()>,
    checkpointed: Result<(), ServeError>,
    inference_shutdown: Result<(), tokio::task::JoinError>,
) -> Result<(), ServeError> {
    served?;
    checkpointed?;
    inference_shutdown.map_err(ServeError::InferenceShutdownTask)
}

/// Checkpoints and flushes every open collection, logging each failure.
async fn checkpoint(state: SharedState) -> Result<(), ServeError> {
    let failures = tokio::task::spawn_blocking(move || state.checkpoint_all())
        .await
        .map_err(ServeError::CheckpointTask)?;
    if failures.is_empty() {
        tracing::info!(target: "piramid::shutdown", "shutdown_checkpointed");
        return Ok(());
    }
    for (collection, error) in &failures {
        tracing::error!(
            target: "piramid::shutdown",
            collection = collection.as_str(),
            %error,
            "shutdown_checkpoint_failed"
        );
    }
    Err(ServeError::Checkpoint(
        failures.into_iter().map(|(name, _)| name).collect(),
    ))
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    reason = "a failed assertion is the point of a test"
)]
mod tests {
    use super::*;
    use piramid_core::config::ApiKey;

    #[test]
    fn loopback_addresses_serve_without_a_key() {
        let auth = AuthConfig::default();
        assert!(check_exposure("127.0.0.1:6333".parse().unwrap(), &auth).is_ok());
        assert!(check_exposure("[::1]:6333".parse().unwrap(), &auth).is_ok());
    }

    #[test]
    fn an_exposed_address_needs_a_key_or_the_explicit_opt_out() {
        let exposed: SocketAddr = "0.0.0.0:6333".parse().unwrap();
        assert!(check_exposure(exposed, &AuthConfig::default()).is_err());

        let with_key = AuthConfig {
            api_key: Some(ApiKey::new("key".to_string()).unwrap()),
            ..AuthConfig::default()
        };
        assert!(check_exposure(exposed, &with_key).is_ok());

        let opted_out = AuthConfig {
            allow_unauthenticated: true,
            ..AuthConfig::default()
        };
        assert!(check_exposure(exposed, &opted_out).is_ok());
    }

    #[tokio::test]
    async fn a_panicked_inference_shutdown_fails_serve_after_the_checkpoint() {
        let panicked = || async {
            let task: tokio::task::JoinHandle<()> =
                tokio::task::spawn_blocking(|| panic!("shutdown"));
            task.await
        };
        assert!(shutdown_outcome(Ok(()), Ok(()), Ok(())).is_ok());
        assert!(matches!(
            shutdown_outcome(Ok(()), Ok(()), panicked().await),
            Err(ServeError::InferenceShutdownTask(_))
        ));
        assert!(matches!(
            shutdown_outcome(
                Ok(()),
                Err(ServeError::Checkpoint(vec![])),
                panicked().await
            ),
            Err(ServeError::Checkpoint(_))
        ));
    }
}
