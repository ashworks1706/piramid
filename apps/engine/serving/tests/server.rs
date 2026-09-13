#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "assertions in tests"
)]
//! Starts the real server on a loopback port and talks to it over TCP.

use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use piramid_core::config::{ApiKey, Config, RateLimitConfig};
use piramid_model::embeddings::EmbeddingsManager;
use piramid_serving::http::serve::{serve, ServeError};
use piramid_serving::state::AppState;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

const KEY: &str = "a-test-key-that-is-long-enough";

/// A server running on its own task, stopped by sending on shutdown.
struct Running {
    addr: SocketAddr,
    shutdown: Option<oneshot::Sender<()>>,
    task: JoinHandle<Result<(), ServeError>>,
}

impl Running {
    fn url(&self, path: &str) -> String {
        format!("http://{}{path}", self.addr)
    }

    async fn stop(mut self) -> Result<(), ServeError> {
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(());
        }
        self.task.await.unwrap()
    }
}

fn data_dir(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("serve_{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn config(dir: &Path) -> Config {
    let mut config = Config::default();
    config.startup.data_dir = dir.to_string_lossy().into_owned();
    config
}

async fn start_on(bind: &str, config: Config) -> Running {
    let state = Arc::new(AppState::new(config, EmbeddingsManager::disabled()).unwrap());
    let listener = TcpListener::bind(bind).await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (tx, rx) = oneshot::channel::<()>();
    let task = tokio::spawn(serve(state, listener, async move {
        let _ = rx.await;
    }));
    Running {
        addr,
        shutdown: Some(tx),
        task,
    }
}

async fn start(config: Config) -> Running {
    start_on("127.0.0.1:0", config).await
}

#[tokio::test]
async fn health_is_open_and_every_other_route_needs_the_key() {
    let dir = data_dir("auth");
    let mut config = config(&dir);
    config.startup.http.auth.api_key = Some(ApiKey::new(KEY.to_string()).unwrap());
    let server = start(config).await;
    let http = reqwest::Client::new();

    for open in ["/api/health", "/api/readyz"] {
        let response = http.get(server.url(open)).send().await.unwrap();
        assert_eq!(response.status(), 200, "{open}");
    }

    for protected in [
        "/api/collections",
        "/api/metrics",
        "/metrics",
        "/api/config",
    ] {
        let missing = http.get(server.url(protected)).send().await.unwrap();
        assert_eq!(missing.status(), 401, "{protected} without a key");
        assert_eq!(
            missing.headers().get("www-authenticate").unwrap(),
            "Bearer",
            "{protected}"
        );

        let wrong = http
            .get(server.url(protected))
            .bearer_auth("not-the-key")
            .send()
            .await
            .unwrap();
        assert_eq!(wrong.status(), 401, "{protected} with the wrong key");

        let right = http
            .get(server.url(protected))
            .bearer_auth(KEY)
            .send()
            .await
            .unwrap();
        assert_eq!(right.status(), 200, "{protected} with the key");
    }

    let not_bearer = http
        .get(server.url("/api/collections"))
        .header("authorization", format!("Basic {KEY}"))
        .send()
        .await
        .unwrap();
    assert_eq!(not_bearer.status(), 401);

    server.stop().await.unwrap();
}

#[tokio::test]
async fn a_loopback_server_with_no_key_serves_without_authentication() {
    let dir = data_dir("loopback_open");
    let server = start(config(&dir)).await;

    let response = reqwest::get(server.url("/api/collections")).await.unwrap();
    assert_eq!(response.status(), 200);

    server.stop().await.unwrap();
}

#[tokio::test]
async fn an_exposed_address_with_no_key_refuses_to_start() {
    let dir = data_dir("exposed");
    let server = start_on("0.0.0.0:0", config(&dir)).await;

    let error = server.task.await.unwrap().unwrap_err();
    let message = error.to_string();
    assert!(message.contains("PIRAMID_API_KEY"), "{message}");
    assert!(message.contains("allow_unauthenticated"), "{message}");
}

#[tokio::test]
async fn an_exposed_address_serves_when_authentication_is_explicitly_off() {
    let dir = data_dir("exposed_opt_out");
    let mut config = config(&dir);
    config.startup.http.auth.allow_unauthenticated = true;
    let server = start_on("0.0.0.0:0", config).await;
    let url = format!("http://127.0.0.1:{}/api/collections", server.addr.port());

    let response = reqwest::get(url).await.unwrap();
    assert_eq!(response.status(), 200);

    server.stop().await.unwrap();
}

#[tokio::test]
async fn a_client_over_its_rate_gets_429_with_retry_after() {
    let dir = data_dir("rate_limit");
    let mut config = config(&dir);
    config.startup.http.rate_limit = Some(RateLimitConfig {
        requests_per_second: 1,
        burst: 2,
    });
    let server = start(config).await;
    let http = reqwest::Client::new();

    for _ in 0..2 {
        let response = http.get(server.url("/api/health")).send().await.unwrap();
        assert_eq!(response.status(), 200);
    }
    let limited = http.get(server.url("/api/health")).send().await.unwrap();
    assert_eq!(limited.status(), 429);
    let retry_after: u64 = limited
        .headers()
        .get("retry-after")
        .unwrap()
        .to_str()
        .unwrap()
        .parse()
        .unwrap();
    assert!(retry_after >= 1, "retry-after {retry_after}");
    let body: serde_json::Value = limited.json().await.unwrap();
    assert_eq!(body["code"], 429);

    server.stop().await.unwrap();
}

fn find_file(dir: &Path, suffix: &str) -> Option<PathBuf> {
    for entry in std::fs::read_dir(dir).ok()? {
        let path = entry.ok()?.path();
        if path.is_dir() {
            if let Some(found) = find_file(&path, suffix) {
                return Some(found);
            }
        } else if path.to_string_lossy().ends_with(suffix) {
            return Some(path);
        }
    }
    None
}

#[tokio::test]
async fn shutdown_finishes_an_in_flight_request_and_checkpoints_the_collection() {
    let dir = data_dir("shutdown");
    let server = start(config(&dir)).await;
    let http = reqwest::Client::new();

    let first = http
        .post(server.url("/api/collections/docs/vectors"))
        .json(&serde_json::json!({"vectors": [[1.0, 0.0, 0.0]], "texts": ["first"]}))
        .send()
        .await
        .unwrap();
    assert_eq!(first.status(), 200);
    assert!(find_file(&dir, ".wal.meta").is_none());

    // Sends the headers and half the body, leaving the request in flight when shutdown starts.
    let body = serde_json::json!({"vectors": [[0.0, 1.0, 0.0]], "texts": ["second"]}).to_string();
    let (head, tail) = body.split_at(body.len() / 2);
    let mut stream = TcpStream::connect(server.addr).await.unwrap();
    let request = format!(
        "POST /api/collections/docs/vectors HTTP/1.1\r\nhost: {}\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{head}",
        server.addr,
        body.len()
    );
    stream.write_all(request.as_bytes()).await.unwrap();
    tokio::time::sleep(Duration::from_millis(200)).await;

    let Running { shutdown, task, .. } = server;
    shutdown.unwrap().send(()).unwrap();
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert!(
        !task.is_finished(),
        "the server stopped with a request in flight"
    );
    assert!(
        TcpStream::connect(format!("127.0.0.1:{}", stream.peer_addr().unwrap().port()))
            .await
            .is_err(),
        "the server accepted a connection after shutdown began"
    );

    stream.write_all(tail.as_bytes()).await.unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).await.unwrap();
    assert!(response.starts_with("HTTP/1.1 200"), "{response}");

    task.await.unwrap().unwrap();
    assert!(find_file(&dir, ".wal.meta").is_some());

    let reopened = AppState::new(config(&dir), EmbeddingsManager::disabled()).unwrap();
    let collection = reopened.get_existing_collection("docs").unwrap();
    assert_eq!(collection.read().count(), 2);
}

/// A server booted from a configuration file, with the state it serves.
async fn start_from_file(file: &Path) -> (Running, Arc<AppState>) {
    use piramid_core::config::loader::{load_from, ConfigSource};

    let source = ConfigSource {
        file: Some(file.to_path_buf()),
        ..ConfigSource::default()
    };
    let config = load_from(&source).unwrap();
    let state = Arc::new(
        AppState::new(config, EmbeddingsManager::disabled())
            .unwrap()
            .with_config_source(source),
    );
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (tx, rx) = oneshot::channel::<()>();
    let task = tokio::spawn(serve(state.clone(), listener, async move {
        let _ = rx.await;
    }));
    (
        Running {
            addr,
            shutdown: Some(tx),
            task,
        },
        state,
    )
}

// A refused reload leaves the configuration and open collections as they were.
#[tokio::test]
async fn a_reload_reaches_open_collections_and_refuses_what_needs_a_reopen() {
    let dir = data_dir("reload");
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("config.yaml");
    let write_config = |runtime: &str| {
        std::fs::write(
            &file,
            format!(
                "startup:\n  data_dir: {}\nruntime:\n{runtime}",
                dir.join("data").display()
            ),
        )
        .unwrap();
    };
    write_config("  search:\n    filter_overfetch: 10\n");
    let (server, state) = start_from_file(&file).await;
    let http = reqwest::Client::new();

    let inserted = http
        .post(server.url("/api/collections/docs/vectors"))
        .json(&serde_json::json!({"vectors": [[1.0, 0.0, 0.0]], "texts": ["first"]}))
        .send()
        .await
        .unwrap();
    assert_eq!(inserted.status(), 200);
    let overfetch = |state: &AppState| {
        state
            .collection_manager
            .get_existing("docs")
            .unwrap()
            .read()
            .config()
            .search
            .filter_overfetch
    };
    assert_eq!(overfetch(&state), 10);

    write_config("  search:\n    filter_overfetch: 3\n");
    let reloaded = http
        .post(server.url("/api/config/reload"))
        .send()
        .await
        .unwrap();
    assert_eq!(reloaded.status(), 200, "{}", reloaded.text().await.unwrap());
    assert_eq!(overfetch(&state), 3, "the open collection took the reload");
    assert_eq!(state.current_config().runtime.search.filter_overfetch, 3);

    write_config(
        "  search:\n    filter_overfetch: 5\n  index:\n    type: flat\n    metric: cosine\n",
    );
    let refused = http
        .post(server.url("/api/config/reload"))
        .send()
        .await
        .unwrap();
    assert_eq!(refused.status(), 400);
    let body = refused.text().await.unwrap();
    assert!(body.contains("runtime.index"), "{body}");
    assert_eq!(overfetch(&state), 3, "a refused reload changes nothing");
    assert_eq!(state.current_config().runtime.search.filter_overfetch, 3);

    server.stop().await.unwrap();
}
