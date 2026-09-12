use dashmap::DashMap;
use parking_lot::RwLock;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    atomic::{AtomicU64, Ordering as AtomicOrdering},
    Arc,
};

use crate::cluster::{
    ClusterRouter, LocalClusterRouter, NodeCapabilities, NodeId, NodeRuntimeState, RouteDecision,
};
use crate::machine::{MachineReadings, SAMPLE_INTERVAL};
use piramid_core::config::{Config, StartupConfig};
use piramid_core::error::{Result, ServerError};
use piramid_database::{CollectionHandle, CollectionManager};
use piramid_model::embeddings::EmbeddingsManager;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RebuildState {
    Running,
    Completed,
    Failed,
}

#[derive(Clone)]
pub struct RebuildJobStatus {
    pub status: RebuildState,
    pub started_at: u64,          // seconds since UNIX epoch
    pub finished_at: Option<u64>, // seconds since UNIX epoch
    pub error: Option<String>,
    pub elapsed_ms: Option<u128>,
}

// Collections are held in a DashMap, one entry per collection.
pub struct AppState {
    pub collection_manager: CollectionManager,
    pub data_dir: String, // e.g. ./data
    pub cluster_router: Arc<dyn ClusterRouter>,
    pub embeddings: EmbeddingsManager,
    /// Readings of the machine the server runs on.
    pub machine: MachineReadings,
    pub shutting_down: Arc<AtomicBool>, // set on shutdown to reject new requests
    pub read_only: Arc<AtomicBool>,     // disk-pressure read-only mode
    pub app_config: Arc<RwLock<Config>>,
    /// The startup block the process booted with. A reload that changes it is refused.
    booted_with: StartupConfig,
    pub rebuild_jobs: Arc<DashMap<String, RebuildJobStatus>>,
    pub config_last_reload: Arc<AtomicU64>, // used to invalidate caches on reload
}

impl AppState {
    pub fn new(config: Config, embeddings: EmbeddingsManager) -> Result<Self> {
        let data_dir = config.startup.data_dir.clone();
        std::fs::create_dir_all(&data_dir)?;
        let booted_with = config.startup.clone();
        let cluster_router: Arc<dyn ClusterRouter> =
            Arc::new(LocalClusterRouter::new(NodeRuntimeState {
                id: NodeId::default(),
                capabilities: NodeCapabilities {
                    cpu_threads: config.startup.threads,
                    memory_budget_bytes: config.startup.hardware.memory_budget_bytes,
                    gpu_enabled: config.startup.hardware.gpu_enabled(),
                },
                healthy: true,
            }));
        let app_config = Arc::new(RwLock::new(config));

        Ok(Self {
            collection_manager: CollectionManager::new(data_dir.clone(), app_config.clone()),
            data_dir,
            cluster_router,
            embeddings,
            machine: MachineReadings::start(SAMPLE_INTERVAL)?,
            shutting_down: Arc::new(AtomicBool::new(false)),
            read_only: Arc::new(AtomicBool::new(false)),
            app_config,
            booted_with,
            rebuild_jobs: Arc::new(DashMap::new()),
            config_last_reload: Arc::new(AtomicU64::new(piramid_core::clock::unix_secs())),
        })
    }

    /// Milliseconds above which a query is logged at warn level.
    pub fn slow_query_ms(&self) -> u128 {
        u128::from(self.booted_with.logging.slow_query_ms())
    }

    pub fn disk_min_free_bytes(&self) -> Option<u64> {
        self.booted_with.disk.min_free_bytes
    }

    pub fn disk_readonly_on_low_space(&self) -> bool {
        self.booted_with.disk.readonly_on_low_space
    }

    pub fn ensure_available(&self) -> Result<()> {
        if self.shutting_down.load(Ordering::Relaxed) {
            return Err(ServerError::ServiceUnavailable("Server is shutting down".into()).into());
        }
        Ok(())
    }

    fn check_routable(&self, name: &str) -> Result<()> {
        self.ensure_available()?;
        if let RouteDecision::Remote(node_id) = self.cluster_router.route_collection(name) {
            return Err(ServerError::ServiceUnavailable(format!(
                "collection '{name}' is assigned to remote node '{node_id}', but remote routing is not implemented"
            ))
            .into());
        }
        Ok(())
    }

    pub fn get_existing_collection(&self, name: &str) -> Result<CollectionHandle> {
        self.check_routable(name)?;
        self.collection_manager.get_existing(name)
    }

    pub fn get_or_create_collection(&self, name: &str) -> Result<CollectionHandle> {
        self.check_routable(name)?;
        self.collection_manager.get_or_create(name)
    }

    pub fn checkpoint_all(&self) -> Result<()> {
        for (_, storage) in self.collection_manager.loaded_collections() {
            let mut storage_guard = storage.write();
            storage_guard.checkpoint()?;
            storage_guard.flush()?;
        }
        Ok(())
    }

    /// Re-read configuration from disk and environment, swapping it in atomically.
    ///
    /// Only the runtime block is swapped. A changed startup block returns an error.
    pub fn reload_config(&self) -> Result<Config> {
        let new_cfg = piramid_core::config::loader::load()
            .map_err(|e| ServerError::InvalidRequest(e.to_string()))?;
        if new_cfg.startup != self.booted_with {
            return Err(ServerError::InvalidRequest(
                "the startup block changed; those settings are applied at boot, so this needs a restart"
                    .to_string(),
            )
            .into());
        }
        {
            let mut guard = self.app_config.write();
            *guard = new_cfg.clone();
        }
        let now = piramid_core::clock::unix_secs();
        self.config_last_reload.store(now, AtomicOrdering::Relaxed);
        Ok(new_cfg)
    }

    pub fn current_config(&self) -> Config {
        self.app_config.read().clone()
    }

    pub fn initiate_shutdown(&self) {
        self.shutting_down.store(true, Ordering::Relaxed);
    }

    fn disk_free_bytes(&self) -> Result<Option<u64>> {
        super::disk::free_bytes(&self.data_dir)
    }

    pub fn ensure_write_allowed(&self) -> Result<()> {
        self.ensure_available()?;
        if self.read_only.load(Ordering::Relaxed) {
            return Err(ServerError::ServiceUnavailable(
                "Server is in read-only mode due to low disk space".into(),
            )
            .into());
        }

        let Some(min_free) = self.disk_min_free_bytes() else {
            return Ok(());
        };
        let Some(free) = self.disk_free_bytes()? else {
            return Ok(());
        };
        if free >= min_free {
            return Ok(());
        }
        if !self.disk_readonly_on_low_space() {
            tracing::warn!(free_bytes = free, min_free = min_free, "disk_space_low");
            return Ok(());
        }
        self.read_only.store(true, Ordering::Relaxed);
        Err(
            ServerError::ServiceUnavailable("Low disk space; write operations disabled".into())
                .into(),
        )
    }

    pub fn enforce_cache_budget(&self) {
        let cache_config = self.current_config().runtime.cache;
        if !cache_config.metadata.enabled {
            return;
        }

        let Some(max_bytes) = cache_config.max_bytes else {
            return;
        };
        let mut total: u64 = 0;
        let mut collections = Vec::new();
        for (name, storage) in self.collection_manager.loaded_collections() {
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
