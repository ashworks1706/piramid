use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use parking_lot::RwLock;
use dashmap::DashMap;
use tokio::runtime::Handle;

use crate::Collection;
use crate::storage::collection::CollectionOpenOptions;
use crate::embeddings::Embedder;
use crate::metrics::LatencyTracker;
use crate::error::{Result, ServerError};
use crate::config::AppConfig;
use std::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};
use std::time::{SystemTime, UNIX_EPOCH};

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
    pub error: Option<String>, // Optional error message if the rebuild job failed
    pub elapsed_ms: Option<u128>, // Optional elapsed time for the rebuild job in milliseconds
}

// Shared application state
// Each collection is an independent Collection with its own file.
// DashMap allows concurrent access to different collections without blocking.
// Holds config + optional embedder so handlers can access without reloading.
pub struct AppState {
    pub collections: DashMap<String, Arc<RwLock<Collection>>>, // Map of collection name to its storage handle. Wrapped in Arc<RwLock> for shared mutable access across threads.
    pub data_dir: String, // Base directory for collection files, e.g. "./data"
    pub embedder: Option<Arc<dyn Embedder>>, // Optional embedder, if configured. Wrapped in Arc for shared ownership.
    pub shutting_down: Arc<AtomicBool>, // Flag to indicate server is shutting down, used to reject new requests gracefully
    pub read_only: Arc<AtomicBool>, // Flag for disk-pressure read-only mode
    pub latency_tracker: Arc<DashMap<String, LatencyTracker>>,  // Per-collection latency tracking
    pub app_config: Arc<RwLock<AppConfig>>, // Global config accessible to handlers, protected by RwLock for dynamic updates
    pub slow_query_ms: u128, // Threshold for logging slow queries in ms
    pub rebuild_jobs: Arc<DashMap<String, RebuildJobStatus>>, // Track index rebuild jobs by collection name
    pub config_last_reload: Arc<AtomicU64>, // Timestamp of last config reload for cache invalidation
    pub disk_min_free_bytes: Option<u64>,
    pub disk_readonly_on_low_space: bool,
    pub cache_max_bytes: Option<u64>,
}

impl AppState {
    pub fn new(
        data_dir: &str,
        app_config: AppConfig,
        slow_query_ms: u128,
        disk_min_free_bytes: Option<u64>,
        disk_readonly_on_low_space: bool,
        cache_max_bytes: Option<u64>,
    ) -> Self {
        std::fs::create_dir_all(data_dir).ok();
        
        Self {
            collections: DashMap::new(),
            data_dir: data_dir.to_string(),
            embedder: None,
            shutting_down: Arc::new(AtomicBool::new(false)),
            read_only: Arc::new(AtomicBool::new(false)),
            latency_tracker: Arc::new(DashMap::new()),
            app_config: Arc::new(RwLock::new(app_config)),
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
            cache_max_bytes,
        }
    }

    pub fn with_embedder(
        data_dir: &str,
        app_config: AppConfig,
        slow_query_ms: u128,
        embedder: Arc<dyn Embedder>,
        disk_min_free_bytes: Option<u64>,
        disk_readonly_on_low_space: bool,
        cache_max_bytes: Option<u64>,
    ) -> Self {
        std::fs::create_dir_all(data_dir).ok();
        
        Self {
            collections: DashMap::new(),
            data_dir: data_dir.to_string(),
            embedder: Some(embedder),
            shutting_down: Arc::new(AtomicBool::new(false)),
            read_only: Arc::new(AtomicBool::new(false)),
            latency_tracker: Arc::new(DashMap::new()),
            app_config: Arc::new(RwLock::new(app_config)),
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
            cache_max_bytes,
        }
    }

    // Lazily load or create a collection
    pub fn get_or_create_collection(&self, name: &str) -> Result<()> {

        if self.shutting_down.load(Ordering::Relaxed) {
            return Err(ServerError::ServiceUnavailable("Server is shutting down".into()).into());
        }

        if !self.collections.contains_key(name) {
            let path = format!("{}/{}.db", self.data_dir, name);
            let cfg = { self.app_config.read().clone() };
            let storage = Collection::open_with_options(
                &path,
                CollectionOpenOptions::from(cfg.to_collection_config()),
            )?;
            let handle = Arc::new(RwLock::new(storage));
            self.collections.insert(name.to_string(), handle.clone());
            
            // Create latency tracker for this collection
            self.latency_tracker.insert(name.to_string(), LatencyTracker::new());

            // Warm caches in the background to avoid first-request latency.
            let warm_handle = handle.clone();
            if let Ok(rt) = Handle::try_current() {
                rt.spawn_blocking(move || {
                    let guard = warm_handle.read();
                    guard.warm_page_cache();
                });
            }
        }
        
        Ok(())
    }

    pub fn checkpoint_all(&self) -> Result<()> {
        for mut entry in self.collections.iter_mut() {
            let storage = entry.value_mut();
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
            return Err(ServerError::ServiceUnavailable("Server is in read-only mode due to low disk space".into()).into());
        }
        if let Some(min_free) = self.disk_min_free_bytes {
            if let Some(free) = self.disk_free_bytes() {
                if free < min_free {
                    if self.disk_readonly_on_low_space {
                        self.read_only.store(true, Ordering::Relaxed);
                        return Err(ServerError::ServiceUnavailable("Low disk space; write operations disabled".into()).into());
                    } else {
                        tracing::warn!(free_bytes=free, min_free=min_free, "disk_space_low");
                    }
                }
            }
        }
        Ok(())
    }

    pub fn enforce_cache_budget(&self) {
        let max_bytes = match self.cache_max_bytes {
            Some(v) => v,
            None => return,
        };
        let mut total: u64 = 0;
        for entry in self.collections.iter() {
            let storage = entry.value();
            let guard = storage.read();
            total = total.saturating_add(guard.cache_usage_bytes() as u64);
        }
        if total > max_bytes {
            tracing::warn!(total_cache_bytes=total, max_bytes=max_bytes, "cache_budget_exceeded_clearing");
            for mut entry in self.collections.iter_mut() {
                let storage = entry.value_mut();
                let mut guard = storage.write();
                guard.clear_caches();
            }
        }
    }
}

pub type SharedState = Arc<AppState>;
