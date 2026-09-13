//! The view the dashboard takes of a running server.
//!
//! Deserialization mirrors of the wire shapes in serving::services::api, holding only the fields
//! the dashboard draws. An unknown field is ignored. A field the server always sends is required,
//! so a body without it is a decode error. A field the server leaves out when it has no value is
//! an Option, and reads as None only when it is absent.

use std::time::Duration;

use piramid_core::config::{ApiKey, API_KEY_ENV};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use serde::Deserialize;

/// Everything one refresh collects.
#[derive(Debug, Clone)]
pub struct Snapshot {
    /// The metrics response.
    pub metrics: Metrics,
    /// The readiness response.
    pub ready: Readyz,
}

/// The version response, read once at startup.
#[derive(Debug, Clone, Deserialize)]
pub struct Version {
    pub version: String,
    pub git_commit: Option<String>,
}

/// The metrics response.
#[derive(Debug, Clone, Deserialize)]
pub struct Metrics {
    pub collections: Vec<CollectionMetrics>,
    pub wal_stats: Vec<WalStats>,
    /// Host readings. None from a server that predates the host block.
    pub host: Option<HostMetrics>,
    /// One entry per GPU the server measured. Empty when the server left the list out.
    #[serde(default)]
    pub gpus: Vec<GpuMetrics>,
    /// Generation counters and scheduler state. None when the server has no model loaded.
    pub inference: Option<InferenceMetrics>,
}

/// Generation counters of the loaded model and the state of its scheduler and key/value cache.
/// An absent average is one the server has not yet measured.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct InferenceMetrics {
    pub model: String,
    pub device: String,
    pub avg_time_to_first_token_ms: Option<f32>,
    pub decode_tokens_per_second: Option<f32>,
    pub preemptions: u64,
    pub queue_depth: u64,
    pub running: u64,
    pub last_batch_size: u64,
    pub kv_blocks_total: u64,
    pub kv_blocks_used: u64,
    pub kv_blocks_cached: u64,
    pub kv_evictions: u64,
    pub prefix_hit_rate: Option<f32>,
}

/// Processor and memory use of the host and of the server process. An absent field is one the
/// server did not measure.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct HostMetrics {
    pub cpu_percent: Option<f32>,
    pub memory_used_bytes: Option<u64>,
    pub memory_total_bytes: Option<u64>,
    pub process_cpu_percent: Option<f32>,
    pub process_resident_bytes: Option<u64>,
}

/// Memory, utilisation and temperature of one GPU. An absent field is one the server did not
/// measure.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct GpuMetrics {
    pub index: u32,
    pub name: Option<String>,
    pub memory_used_bytes: Option<u64>,
    pub memory_total_bytes: Option<u64>,
    pub utilization_percent: Option<f32>,
    pub temperature_celsius: Option<f32>,
}

/// The counters of one collection.
///
/// The server sends every Option here as null when it has no value, so each key is required.
#[derive(Debug, Clone, Deserialize)]
pub struct CollectionMetrics {
    pub name: String,
    pub vector_count: usize,
    pub index_type: String,
    pub memory_usage_bytes: usize,
    #[serde(deserialize_with = "Option::deserialize")]
    pub insert_latency_ms: Option<f32>,
    #[serde(deserialize_with = "Option::deserialize")]
    pub search_latency_ms: Option<f32>,
    #[serde(deserialize_with = "Option::deserialize")]
    pub lock_read_ms: Option<f32>,
    #[serde(deserialize_with = "Option::deserialize")]
    pub lock_write_ms: Option<f32>,
    #[serde(deserialize_with = "Option::deserialize")]
    pub hnsw_ef_search: Option<usize>,
    #[serde(deserialize_with = "Option::deserialize")]
    pub ivf_nprobe: Option<usize>,
}

/// The durability state of one collection.
///
/// The server sends every Option here as null when it has no value, so each key is required.
#[derive(Debug, Clone, Deserialize)]
pub struct WalStats {
    pub collection: String,
    #[serde(deserialize_with = "Option::deserialize")]
    pub checkpoint_age_secs: Option<u64>,
    #[serde(deserialize_with = "Option::deserialize")]
    pub wal_size_bytes: Option<u64>,
}

/// The readiness response.
#[derive(Debug, Clone, Deserialize)]
pub struct Readyz {
    pub collections: Vec<CollectionHealth>,
}

/// One collection as readiness sees it.
#[derive(Debug, Clone, Deserialize)]
pub struct CollectionHealth {
    pub name: String,
    pub loaded: bool,
    pub integrity_ok: Option<bool>,
    pub error: Option<String>,
}

/// Where a rebuild is, from the rebuild status endpoint.
#[derive(Debug, Clone, Deserialize)]
pub struct RebuildStatus {
    pub status: String,
    pub error: Option<String>,
}

/// Anything that stopped a request from answering.
#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    /// The server could not be reached, or did not answer in time.
    #[error("{0} unreachable: {1}")]
    Unreachable(String, String),
    /// The server refused the request for a missing or wrong API key.
    #[error("{path} was refused: {reason}")]
    Unauthorized {
        /// The request path.
        path: String,
        /// What to change.
        reason: String,
    },
    /// The server answered with a status other than 2xx.
    #[error("{0} returned {1}: {2}")]
    Status(String, u16, String),
    /// The body did not match the shape the dashboard expects.
    #[error("{0}: {1}")]
    Decode(String, String),
    /// The HTTP client itself could not be built.
    #[error("http client: {0}")]
    Build(String),
}

/// An HTTP client bound to one server.
#[derive(Debug, Clone)]
pub struct Client {
    http: reqwest::Client,
    base: String,
    keyed: bool,
}

impl Client {
    /// A client for base, with a request timeout so a hung server does not freeze the UI.
    ///
    /// A key is sent as a bearer token on every request.
    pub fn new(base: &str, timeout: Duration, key: Option<ApiKey>) -> Result<Self, ClientError> {
        let mut headers = HeaderMap::new();
        if let Some(key) = &key {
            let mut value =
                HeaderValue::from_str(&format!("Bearer {}", key.expose())).map_err(|_| {
                    ClientError::Build(format!("{API_KEY_ENV} is not a valid header value"))
                })?;
            value.set_sensitive(true);
            headers.insert(AUTHORIZATION, value);
        }
        let http = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(2))
            .timeout(timeout)
            .default_headers(headers)
            .build()
            .map_err(|e| ClientError::Build(e.to_string()))?;
        Ok(Self {
            http,
            base: base.trim_end_matches('/').to_owned(),
            keyed: key.is_some(),
        })
    }

    /// Build identity, read once.
    pub async fn version(&self) -> Result<Version, ClientError> {
        self.get("/api/version").await
    }

    /// One refresh. Metrics and readiness are requested concurrently.
    pub async fn snapshot(&self) -> Result<Snapshot, ClientError> {
        let (metrics, ready) = tokio::join!(self.get("/api/metrics"), self.get("/api/readyz"));
        Ok(Snapshot {
            metrics: metrics?,
            ready: ready?,
        })
    }

    /// The configuration the server resolved, rendered as YAML.
    ///
    /// The response nests the configuration under the app_config key, which is unwrapped here so
    /// the view shows the same shape as the file on disk. A response without that key is a
    /// decode error.
    pub async fn config(&self) -> Result<String, ClientError> {
        let value: serde_json::Value = self.get(CONFIG_PATH).await?;
        render_config(&value)
    }

    /// Asks for an index rebuild and returns once the server accepts it.
    pub async fn rebuild(&self, collection: &str) -> Result<(), ClientError> {
        self.post_empty(&format!("/api/collections/{collection}/index/rebuild"))
            .await
    }

    /// Where a rebuild started earlier has got to.
    pub async fn rebuild_status(&self, collection: &str) -> Result<RebuildStatus, ClientError> {
        self.get(&format!(
            "/api/collections/{collection}/index/rebuild/status"
        ))
        .await
    }

    /// Compacts a collection, reclaiming space held by deleted records.
    pub async fn compact(&self, collection: &str) -> Result<(), ClientError> {
        self.post_empty(&format!("/api/collections/{collection}/compact"))
            .await
    }

    async fn get<T: serde::de::DeserializeOwned>(&self, path: &str) -> Result<T, ClientError> {
        let response = self
            .http
            .get(format!("{}{path}", self.base))
            .send()
            .await
            .map_err(|e| ClientError::Unreachable(path.to_owned(), root_cause(&e)))?;
        self.decode(path, response).await
    }

    async fn post_empty(&self, path: &str) -> Result<(), ClientError> {
        let response = self
            .http
            .post(format!("{}{path}", self.base))
            .json(&serde_json::json!({}))
            .send()
            .await
            .map_err(|e| ClientError::Unreachable(path.to_owned(), root_cause(&e)))?;
        let _: serde_json::Value = self.decode(path, response).await?;
        Ok(())
    }

    async fn decode<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        response: reqwest::Response,
    ) -> Result<T, ClientError> {
        let status = response.status();
        if status == reqwest::StatusCode::UNAUTHORIZED {
            let reason = if self.keyed {
                format!("the server rejected the key in {API_KEY_ENV}")
            } else {
                format!("the server requires an API key; set {API_KEY_ENV}")
            };
            return Err(ClientError::Unauthorized {
                path: path.to_owned(),
                reason,
            });
        }
        let body = response
            .text()
            .await
            .map_err(|e| ClientError::Unreachable(path.to_owned(), root_cause(&e)))?;
        if !status.is_success() {
            return Err(ClientError::Status(
                path.to_owned(),
                status.as_u16(),
                summarize(&body),
            ));
        }
        parse(path, &body)
    }
}

/// The path of the resolved configuration.
const CONFIG_PATH: &str = "/api/config";

/// Decodes the body path answered with into the shape T.
pub fn parse<T: serde::de::DeserializeOwned>(path: &str, body: &str) -> Result<T, ClientError> {
    serde_json::from_str(body).map_err(|e| ClientError::Decode(path.to_owned(), e.to_string()))
}

/// The configuration under the app_config key of a config response, rendered as YAML.
pub fn render_config(response: &serde_json::Value) -> Result<String, ClientError> {
    let config = response.get("app_config").ok_or_else(|| {
        ClientError::Decode(
            CONFIG_PATH.to_owned(),
            "the response has no app_config field".to_owned(),
        )
    })?;
    yaml_serde::to_string(config)
        .map_err(|e| ClientError::Decode(CONFIG_PATH.to_owned(), e.to_string()))
}

/// The innermost reason a request failed, such as the refused connection or the timeout.
pub fn root_cause(error: &reqwest::Error) -> String {
    let mut source: &dyn std::error::Error = error;
    while let Some(inner) = source.source() {
        source = inner;
    }
    source.to_string()
}

/// An error body trimmed to something that fits on the status line.
fn summarize(body: &str) -> String {
    let text = serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|v| {
            v.get("error")
                .or_else(|| v.get("message"))
                .and_then(|m| m.as_str().map(str::to_owned))
        })
        .unwrap_or_else(|| body.trim().to_owned());
    text.chars().take(140).collect()
}
