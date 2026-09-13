//! Periodic probes of the server and the website.

use tokio::sync::mpsc::UnboundedSender;

use crate::console::client::root_cause;
use crate::console::settings::Settings;
use crate::console::types::{Event, Health, Probe};

/// What to probe and how often.
#[derive(Debug, Clone)]
pub struct Targets {
    live: String,
    ready: String,
    web: String,
    every: std::time::Duration,
}

impl Targets {
    /// Targets from settings.
    pub fn from_settings(settings: &Settings) -> Self {
        let base = settings.base_url.trim_end_matches('/');
        Self {
            live: format!("{base}/api/health"),
            ready: format!("{base}/api/readyz"),
            web: settings.web_url.clone(),
            every: settings.refresh,
        }
    }
}

/// Probes forever on the configured interval, sending each result to the UI.
///
/// The website probe is skipped where there is no checkout to serve one from. An HTTP client that
/// cannot be built is sent to the UI as the reason no probe runs.
pub async fn poll(targets: Targets, probe_web: bool, tx: UnboundedSender<Event>) {
    let http = match reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(2))
        // Readiness opens every collection on disk and can be slow on a large data directory.
        .timeout(std::time::Duration::from_secs(8))
        .build()
    {
        Ok(http) => http,
        Err(e) => {
            let _ = tx.send(Event::ProbesStopped(format!(
                "health probes are off: http client: {}",
                root_cause(&e)
            )));
            return;
        }
    };
    loop {
        let (live, ready) = tokio::join!(probe(&http, &targets.live), probe(&http, &targets.ready));
        let web = if probe_web {
            probe(&http, &targets.web).await
        } else {
            Probe::Unknown
        };
        if tx
            .send(Event::Health(Box::new(Health { live, ready, web })))
            .is_err()
        {
            return;
        }
        tokio::time::sleep(targets.every).await;
    }
}

async fn probe(http: &reqwest::Client, url: &str) -> Probe {
    match http.get(url).send().await {
        Ok(response) if response.status().is_success() => Probe::Up,
        Ok(response) => {
            let status = response.status();
            match response.text().await {
                Ok(body) => Probe::Degraded(format!(
                    "{status}: {}",
                    body.chars().take(120).collect::<String>()
                )),
                Err(e) => Probe::Degraded(format!("{status}: body unreadable: {}", root_cause(&e))),
            }
        }
        Err(e) => Probe::Down(root_cause(&e)),
    }
}
