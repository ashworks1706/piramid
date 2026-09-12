//! Process-wide shared state of the server and the index rebuild job registry.

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
use piramid_core::config::loader::ConfigSource;
use piramid_core::config::{Config, HttpConfig, StartupConfig};
use piramid_core::error::{PiramidError, Result, ServerError};
use piramid_database::{CollectionHandle, CollectionManager};
use piramid_model::embeddings::EmbeddingsManager;

/// Phase of an index rebuild job.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RebuildState {
    /// The rebuild has started and not yet finished.
    Running,
    /// The rebuild finished without error.
    Completed,
    /// The rebuild returned an error.
    Failed,
}

/// Record of the most recent index rebuild of one collection.
#[derive(Clone)]
pub struct RebuildJobStatus {
    /// Current phase of the job.
    pub status: RebuildState,
    /// Start time, in seconds since the Unix epoch.
    pub started_at: u64,
    /// End time, in seconds since the Unix epoch. None while running.
    pub finished_at: Option<u64>,
    /// Error message of a failed rebuild. None unless failed.
    pub error: Option<String>,
    /// Duration of the rebuild, in milliseconds. None while running.
    pub elapsed_ms: Option<u128>,
}

/// Process-wide state shared by every request handler.
pub struct AppState {
    /// Open collections and their latency trackers.
    pub collection_manager: CollectionManager,
    /// Directory holding the collection files.
    pub data_dir: String,
    /// Decides which node serves each collection.
    pub cluster_router: Arc<dyn ClusterRouter>,
    /// The embedding provider, if configured, and its usage metrics.
    pub embeddings: EmbeddingsManager,
    /// Readings of the machine the server runs on.
    pub machine: MachineReadings,
    /// Set once shutdown begins; requests are refused with 503 afterwards.
    pub shutting_down: Arc<AtomicBool>,
    /// Set when low disk space disables writes; writes are refused with 503 afterwards.
    pub read_only: Arc<AtomicBool>,
    /// The configuration in effect, replaced on reload.
    pub app_config: Arc<RwLock<Config>>,
    /// The startup block the process booted with. A reload that changes it is refused.
    booted_with: StartupConfig,
    /// Where a reload reads configuration from.
    config_source: ConfigSource,
    /// Most recent index rebuild of each collection, keyed by collection name.
    pub rebuild_jobs: Arc<DashMap<String, RebuildJobStatus>>,
    /// Time of startup or of the last successful reload, in seconds since the Unix epoch.
    pub config_last_reload: Arc<AtomicU64>,
}

impl AppState {
    /// Build the state from a loaded configuration, creating the data directory if missing.
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
            config_source: ConfigSource::default(),
            rebuild_jobs: Arc::new(DashMap::new()),
            config_last_reload: Arc::new(AtomicU64::new(piramid_core::clock::unix_secs())),
        })
    }

    /// Read reloads from source, the one the process booted from.
    pub fn with_config_source(mut self, source: ConfigSource) -> Self {
        self.config_source = source;
        self
    }

    /// Authentication, rate limiting and shutdown settings the process booted with.
    pub fn http_config(&self) -> &HttpConfig {
        &self.booted_with.http
    }

    /// Milliseconds above which a query is logged at warn level.
    pub fn slow_query_ms(&self) -> u128 {
        u128::from(self.booted_with.logging.slow_query_ms)
    }

    /// Free bytes below which the disk counts as low, from the startup config.
    pub fn disk_min_free_bytes(&self) -> Option<u64> {
        self.booted_with.disk.min_free_bytes
    }

    /// Whether low disk space switches the server to read-only, from the startup config.
    pub fn disk_readonly_on_low_space(&self) -> bool {
        self.booted_with.disk.readonly_on_low_space
    }

    /// Error with 503 once shutdown has begun.
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

    /// Handle to a collection that is loaded or present on disk, opening it if needed.
    pub fn get_existing_collection(&self, name: &str) -> Result<CollectionHandle> {
        self.check_routable(name)?;
        self.collection_manager.get_existing(name)
    }

    /// Handle to a collection, opening or creating it if needed.
    pub fn get_or_create_collection(&self, name: &str) -> Result<CollectionHandle> {
        self.check_routable(name)?;
        self.collection_manager.get_or_create(name)
    }

    /// Checkpoints and flushes every loaded collection, returning the name and error of each that
    /// failed.
    pub fn checkpoint_all(&self) -> Vec<(String, PiramidError)> {
        let mut failures = Vec::new();
        for (name, storage) in self.collection_manager.loaded_collections() {
            let mut storage_guard = storage.write();
            if let Err(error) = storage_guard
                .checkpoint()
                .and_then(|()| storage_guard.flush())
            {
                failures.push((name, error));
            }
        }
        failures
    }

    /// Re-read configuration from the boot source, and apply its runtime block to the process
    /// and to every open collection.
    ///
    /// A changed startup block, or a change an open collection can take only when it is opened,
    /// refuses the reload and changes nothing.
    pub fn reload_config(&self) -> Result<Config> {
        let new_cfg = piramid_core::config::loader::load_from(&self.config_source)
            .map_err(|e| ServerError::InvalidRequest(e.to_string()))?;
        if new_cfg.startup != self.booted_with {
            return Err(ServerError::InvalidRequest(
                "the startup block changed; those settings are applied at boot, so this needs a restart"
                    .to_string(),
            )
            .into());
        }
        let next = new_cfg.to_collection_config();
        for (name, handle) in self.collection_manager.loaded_collections() {
            if let Some(setting) = handle.read().setting_needing_reopen(&next) {
                return Err(ServerError::InvalidRequest(format!(
                    "{setting} changed, and collection '{name}' is open; it applies when a \
                     collection is opened, so this needs a restart"
                ))
                .into());
            }
        }
        {
            let mut guard = self.app_config.write();
            *guard = new_cfg.clone();
        }
        // A collection opened after the swap already has the new configuration.
        for (name, handle) in self.collection_manager.loaded_collections() {
            handle.write().apply_live_settings(&next).map_err(|error| {
                ServerError::Internal(format!(
                    "collection '{name}' opened while the reload was applied and refused it: {error}"
                ))
            })?;
        }
        let now = piramid_core::clock::unix_secs();
        self.config_last_reload.store(now, AtomicOrdering::Relaxed);
        Ok(new_cfg)
    }

    /// Copy of the configuration in effect.
    pub fn current_config(&self) -> Config {
        self.app_config.read().clone()
    }

    /// Mark the server as shutting down.
    pub fn initiate_shutdown(&self) {
        self.shutting_down.store(true, Ordering::Relaxed);
    }

    fn disk_free_bytes(&self) -> Result<Option<u64>> {
        super::disk::free_bytes(&self.data_dir)
    }

    /// Error with 503 when shutting down, read-only, or below the free-space floor with read-only
    /// on low space enabled. Enabling read-only here latches it.
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

    /// Clear the largest metadata caches until cached metadata fits runtime.cache.metadata.max_bytes.
    pub fn enforce_cache_budget(&self) {
        let metadata_config = self.current_config().runtime.cache.metadata;
        let Some(max_bytes) = metadata_config.max_bytes else {
            return;
        };
        let mut total: u64 = 0;
        let mut collections = Vec::new();
        for (name, storage) in self.collection_manager.loaded_collections() {
            let metadata_bytes = storage.read().metadata_cache_usage_bytes() as u64;
            total = total.saturating_add(metadata_bytes);
            collections.push((name, storage, metadata_bytes));
        }
        if total <= max_bytes {
            return;
        }
        tracing::warn!(
            target: "piramid::cache",
            metadata_bytes = total,
            max_bytes = max_bytes,
            "metadata_cache_budget_exceeded"
        );
        collections.sort_by_key(|collection| std::cmp::Reverse(collection.2));
        for (name, storage, metadata_bytes) in collections {
            if total <= max_bytes || metadata_bytes == 0 {
                break;
            }
            let freed = storage.write().clear_metadata_cache() as u64;
            total = total.saturating_sub(freed);
            tracing::debug!(
                target: "piramid::cache",
                collection = name,
                freed_bytes = freed,
                metadata_bytes = total,
                "metadata_cache_cleared"
            );
        }
    }
}

/// Reference-counted handle to [AppState].
pub type SharedState = Arc<AppState>;
