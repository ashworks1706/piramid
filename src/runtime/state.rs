use dashmap::DashMap;
use parking_lot::RwLock;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    atomic::{AtomicU64, Ordering as AtomicOrdering},
    Arc,
};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::collections::{CollectionHandle, CollectionRegistry};
use crate::config::AppConfig;
use crate::embeddings::Embedder;
use crate::error::{Result, ServerError};
use crate::metrics::EmbedMetrics;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RebuildState {
    Running,
    Completed,
    Failed,
}

#[derive(Clone)]
pub struct RebuildJobStatus {
    pub status: RebuildState, // Current status of the rebuild job (Running, Completed, Failed)
    pub started_at: u64, // Timestamp when the rebuild job started (in seconds since UNIX epoch)
    pub finished_at: Option<u64>, // Optional timestamp when the rebuild job finished (in seconds since UNIX epoch)
    pub error: Option<String>,    // Optional error message if the rebuild job failed
    pub elapsed_ms: Option<u128>, // Optional elapsed time for the rebuild job in milliseconds
}

// Shared application state
// Each collection is an independent Collection with its own file.
// DashMap allows concurrent access to different collections without blocking.
// Holds config + optional embedder so handlers can access without reloading.
pub struct AppState {
    pub registry: CollectionRegistry,
    pub data_dir: String, // Base directory for collection files, e.g. "./data"
    pub embedder: Option<Arc<dyn Embedder>>, // Optional embedder, if configured. Wrapped in Arc for shared ownership.
    pub shutting_down: Arc<AtomicBool>, // Flag to indicate server is shutting down, used to reject new requests gracefully
    pub read_only: Arc<AtomicBool>,     // Flag for disk-pressure read-only mode
    pub embed_metrics: Arc<EmbedMetrics>,
    pub app_config: Arc<RwLock<AppConfig>>, // Global config accessible to handlers, protected by RwLock for dynamic updates
    pub slow_query_ms: u128,                // Threshold for logging slow queries in ms
    pub rebuild_jobs: Arc<DashMap<String, RebuildJobStatus>>, // Track index rebuild jobs by collection name
    pub config_last_reload: Arc<AtomicU64>, // Timestamp of last config reload for cache invalidation
    pub disk_min_free_bytes: Option<u64>,
    pub disk_readonly_on_low_space: bool,
}

impl AppState {
    pub fn new(
        data_dir: &str,
        app_config: AppConfig,
        slow_query_ms: u128,
        disk_min_free_bytes: Option<u64>,
        disk_readonly_on_low_space: bool,
    ) -> Self {
        std::fs::create_dir_all(data_dir).ok();
        let app_config = Arc::new(RwLock::new(app_config));

        Self {
            registry: CollectionRegistry::new(data_dir.to_string(), app_config.clone()),
            data_dir: data_dir.to_string(),
            embedder: None,
            shutting_down: Arc::new(AtomicBool::new(false)),
            read_only: Arc::new(AtomicBool::new(false)),
            embed_metrics: Arc::new(EmbedMetrics::default()),
            app_config,
            slow_query_ms,
            rebuild_jobs: Arc::new(DashMap::new()),
            // Initialize to current time; updated on each config reload
            config_last_reload: Arc::new(AtomicU64::new(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            )),
            disk_min_free_bytes,
            disk_readonly_on_low_space,
        }
    }

    pub fn with_embedder(
        data_dir: &str,
        app_config: AppConfig,
        slow_query_ms: u128,
        embedder: Arc<dyn Embedder>,
        disk_min_free_bytes: Option<u64>,
        disk_readonly_on_low_space: bool,
    ) -> Self {
        std::fs::create_dir_all(data_dir).ok();
        let app_config = Arc::new(RwLock::new(app_config));

        Self {
            registry: CollectionRegistry::new(data_dir.to_string(), app_config.clone()),
            data_dir: data_dir.to_string(),
            embedder: Some(embedder),
            shutting_down: Arc::new(AtomicBool::new(false)),
            read_only: Arc::new(AtomicBool::new(false)),
            embed_metrics: Arc::new(EmbedMetrics::default()),
            app_config,
            slow_query_ms,
            rebuild_jobs: Arc::new(DashMap::new()),
            config_last_reload: Arc::new(AtomicU64::new(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            )),
            disk_min_free_bytes,
            disk_readonly_on_low_space,
        }
    }

    pub fn get_existing_collection(&self, name: &str) -> Result<CollectionHandle> {
        if self.shutting_down.load(Ordering::Relaxed) {
            return Err(ServerError::ServiceUnavailable("Server is shutting down".into()).into());
        }
        self.registry.get_existing(name)
    }

    // Lazily load or create a collection
    pub fn get_or_create_collection(&self, name: &str) -> Result<CollectionHandle> {
        if self.shutting_down.load(Ordering::Relaxed) {
            return Err(ServerError::ServiceUnavailable("Server is shutting down".into()).into());
        }
        self.registry.get_or_create(name)
    }

    pub fn checkpoint_all(&self) -> Result<()> {
        for (_, storage) in self.registry.loaded_collections() {
            let mut storage_guard = storage.write();
            storage_guard.checkpoint()?;
            storage_guard.flush()?;
        }
        Ok(())
    }

    pub fn reload_config(&self) -> Result<AppConfig> {
        let new_cfg = crate::config::loader::load_app_config();
        {
            let mut guard = self.app_config.write();
            *guard = new_cfg.clone();
        }
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        self.config_last_reload.store(now, AtomicOrdering::Relaxed);
        Ok(new_cfg)
    }

    pub fn current_config(&self) -> AppConfig {
        self.app_config.read().clone()
    }

    pub fn initiate_shutdown(&self) {
        self.shutting_down.store(true, Ordering::Relaxed);
    }

    fn disk_free_bytes(&self) -> Option<u64> {
        #[cfg(target_family = "unix")]
        {
            use std::ffi::CString;
            let c_path = CString::new(self.data_dir.clone()).ok()?;
            let mut stat: libc::statvfs = unsafe { std::mem::zeroed() };
            let rc = unsafe { libc::statvfs(c_path.as_ptr(), &mut stat) };
            if rc == 0 {
                let avail = (stat.f_bavail as u64).saturating_mul(stat.f_frsize as u64);
                return Some(avail);
            }
        }
        None
    }

    pub fn ensure_write_allowed(&self) -> Result<()> {
        if self.shutting_down.load(Ordering::Relaxed) {
            return Err(ServerError::ServiceUnavailable("Server is shutting down".into()).into());
        }
        if self.read_only.load(Ordering::Relaxed) {
            return Err(ServerError::ServiceUnavailable(
                "Server is in read-only mode due to low disk space".into(),
            )
            .into());
        }
        if let Some(min_free) = self.disk_min_free_bytes {
            if let Some(free) = self.disk_free_bytes() {
                if free < min_free {
                    if self.disk_readonly_on_low_space {
                        self.read_only.store(true, Ordering::Relaxed);
                        return Err(ServerError::ServiceUnavailable(
                            "Low disk space; write operations disabled".into(),
                        )
                        .into());
                    } else {
                        tracing::warn!(free_bytes = free, min_free = min_free, "disk_space_low");
                    }
                }
            }
        }
        Ok(())
    }

    pub fn enforce_cache_budget(&self) {
        let cache_config = self.current_config().cache;
        if !cache_config.enabled {
            return;
        }

        let max_bytes = match cache_config.max_bytes {
            Some(v) => v,
            None => return,
        };
        let mut total: u64 = 0;
        let mut collections = Vec::new();
        for (name, storage) in self.registry.loaded_collections() {
            let guard = storage.read();
            let cache_bytes = guard.cache_usage_bytes();
            let metadata_bytes = guard.metadata_cache_usage_bytes();
            total = total.saturating_add(cache_bytes as u64);
            collections.push((name, storage.clone(), metadata_bytes));
        }

        if total > max_bytes {
            tracing::warn!(
                total_cache_bytes = total,
                max_bytes = max_bytes,
                "cache_budget_exceeded_evicting_metadata"
            );

            collections.sort_by_key(|collection| std::cmp::Reverse(collection.2));
            for (name, storage, metadata_bytes) in collections {
                if total <= max_bytes || metadata_bytes == 0 {
                    break;
                }
                let mut guard = storage.write();
                let freed = guard.clear_metadata_cache() as u64;
                total = total.saturating_sub(freed);
                tracing::debug!(
                    collection = name,
                    freed_cache_bytes = freed,
                    total_cache_bytes = total,
                    "metadata_cache_evicted"
                );
            }
        }
    }
}

pub type SharedState = Arc<AppState>;
