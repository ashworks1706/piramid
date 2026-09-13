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
    /// The server version.
    pub version: String,
    /// The commit the server was built from, if known.
    pub git_commit: Option<String>,
}

/// The metrics response.
#[derive(Debug, Clone, Deserialize)]
pub struct Metrics {
    /// The counters of each open collection.
    pub collections: Vec<CollectionMetrics>,
    /// The durability state of each open collection.
    pub wal_stats: Vec<WalStats>,
    /// Host readings.
    pub host: HostMetrics,
    /// One entry per GPU the server measured. Empty when the server left the list out.
    #[serde(default)]
    pub gpus: Vec<GpuMetrics>,
    /// How the device memory budget is divided and used. None when the server has no GPU open.
    pub gpu_budget: Option<GpuBudget>,
    /// Generation counters and scheduler state. None when the server has no model loaded.
    pub inference: Option<InferenceMetrics>,
}

/// Generation counters of the loaded model and the state of its scheduler and key/value cache.
/// An absent average is one the server has not yet measured.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct InferenceMetrics {
    /// Checkpoint name of the loaded model.
    pub model: String,
    /// The device the model runs on.
    pub device: String,
    /// Mean time from admission to the first token, in milliseconds.
    pub avg_time_to_first_token_ms: Option<f32>,
    /// Decode tokens per second.
    pub decode_tokens_per_second: Option<f32>,
    /// Sequences preempted for recompute.
    pub preemptions: u64,
    /// Requests waiting for admission.
    pub queue_depth: u64,
    /// Sequences being generated.
    pub running: u64,
    /// Sequences in the most recent step.
    pub last_batch_size: u64,
    /// Pages in the key/value pool.
    pub kv_blocks_total: u64,
    /// Pages held by a sequence.
    pub kv_blocks_used: u64,
    /// Free pages still carrying a reusable prefix.
    pub kv_blocks_cached: u64,
    /// Prefix pages evicted to make room.
    pub kv_evictions: u64,
    /// Share of looked-up prompt tokens served from shared pages.
    pub prefix_hit_rate: Option<f32>,
}

/// The device memory budget: the bytes it covers and the use of each pool drawing from it.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct GpuBudget {
    /// Bytes the budget covers after the reserve.
    pub usable_bytes: u64,
    /// Whether every pool draws from one shared budget.
    pub shared: bool,
    /// Capacity and use of each pool, in the order the server sends them.
    pub pools: Vec<GpuPool>,
}

/// One pool of the device memory budget. Under a shared budget the capacity is the whole budget.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct GpuPool {
    /// The pool name: weights, kv_cache or index.
    pub pool: String,
    /// Bytes the pool may use.
    pub capacity_bytes: u64,
    /// Bytes the pool uses.
    pub used_bytes: u64,
}

/// Processor and memory use of the host and of the server process. An absent field is one the
/// server did not measure.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct HostMetrics {
    /// Processor use of the whole host, 0 to 100.
    pub cpu_percent: Option<f32>,
    /// Host memory in use, in bytes.
    pub memory_used_bytes: Option<u64>,
    /// Host memory installed, in bytes.
    pub memory_total_bytes: Option<u64>,
    /// Server process processor use, in percent.
    pub process_cpu_percent: Option<f32>,
    /// Server process resident memory, in bytes.
    pub process_resident_bytes: Option<u64>,
}

/// Memory, utilisation and temperature of one GPU. An absent field is one the server did not
/// measure.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct GpuMetrics {
    /// The device ordinal.
    pub index: u32,
    /// The device name.
    pub name: Option<String>,
    /// Device memory in use, in bytes.
    pub memory_used_bytes: Option<u64>,
    /// Device memory installed, in bytes.
    pub memory_total_bytes: Option<u64>,
    /// Device utilisation, 0 to 100.
    pub utilization_percent: Option<f32>,
    /// Device temperature, in degrees Celsius.
    pub temperature_celsius: Option<f32>,
}

/// The counters of one collection.
///
/// The server sends every Option here as null when it has no value, and each key is required.
#[derive(Debug, Clone, Deserialize)]
pub struct CollectionMetrics {
    /// The collection name.
    pub name: String,
    /// Number of stored documents.
    pub vector_count: usize,
    /// Index family: Flat, HNSW or IVF.
    pub index_type: String,
    /// Approximate resident size of records, offsets, caches and index, in bytes.
    pub memory_usage_bytes: usize,
    /// Moving average of insert duration, in milliseconds.
    #[serde(deserialize_with = "Option::deserialize")]
    pub insert_latency_ms: Option<f32>,
    /// Moving average of search duration, in milliseconds.
    #[serde(deserialize_with = "Option::deserialize")]
    pub search_latency_ms: Option<f32>,
    /// Moving average of read-lock wait, in milliseconds.
    #[serde(deserialize_with = "Option::deserialize")]
    pub lock_read_ms: Option<f32>,
    /// Moving average of write-lock wait, in milliseconds.
    #[serde(deserialize_with = "Option::deserialize")]
    pub lock_write_ms: Option<f32>,
    /// Configured HNSW candidate-list width.
    #[serde(deserialize_with = "Option::deserialize")]
    pub hnsw_ef_search: Option<usize>,
    /// Configured IVF partitions to scan.
    #[serde(deserialize_with = "Option::deserialize")]
    pub ivf_nprobe: Option<usize>,
}

/// The durability state of one collection.
///
/// The server sends every Option here as null when it has no value, and each key is required.
#[derive(Debug, Clone, Deserialize)]
pub struct WalStats {
    /// The collection name.
    pub collection: String,
    /// Seconds since the last checkpoint.
    #[serde(deserialize_with = "Option::deserialize")]
    pub checkpoint_age_secs: Option<u64>,
    /// Size of the write-ahead log file, in bytes.
    #[serde(deserialize_with = "Option::deserialize")]
    pub wal_size_bytes: Option<u64>,
}

/// The readiness response.
#[derive(Debug, Clone, Deserialize)]
pub struct Readyz {
    /// The health of each collection, loaded or on disk only.
    pub collections: Vec<CollectionHealth>,
}

/// One collection as readiness sees it.
#[derive(Debug, Clone, Deserialize)]
pub struct CollectionHealth {
    /// The collection name.
    pub name: String,
    /// True when the collection is open in memory.
    pub loaded: bool,
    /// Whether the collection passed its integrity check, when reported.
    pub integrity_ok: Option<bool>,
    /// The error reported for the collection, if any.
    pub error: Option<String>,
}

/// Where a rebuild is, from the rebuild status endpoint.
#[derive(Debug, Clone, Deserialize)]
pub struct RebuildStatus {
    /// One of running, completed or failed.
    pub status: String,
    /// Why the rebuild failed, if it did.
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
    /// A client for base, with a request timeout.
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
    /// The configuration is unwrapped from the app_config key of the response. A response without
    /// that key is a decode error.
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
pub fn summarize(body: &str) -> String {
    let text = serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|v| v.get("error").and_then(|m| m.as_str().map(str::to_owned)))
        .unwrap_or_else(|| body.trim().to_owned());
    text.chars().take(140).collect()
}
