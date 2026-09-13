//! Process-wide shared state of the server.

use parking_lot::RwLock;
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc,
};

use crate::machine::{MachineReadings, SAMPLE_INTERVAL};
use piramid_core::config::loader::ConfigSource;
use piramid_core::config::InferenceConfig;
use piramid_core::config::{Config, HttpConfig, StartupConfig};
use piramid_core::error::{PiramidError, Result, ServerError};
use piramid_database::{CollectionHandle, CollectionManager};
use piramid_hardware::gpu::GpuManager;
use piramid_model::embeddings::EmbeddingsManager;
use piramid_model::inference::InferenceManager;

/// Process-wide state shared by every request handler.
pub struct AppState {
    /// Open collections and their latency trackers.
    pub collection_manager: CollectionManager,
    /// Directory holding the collection files.
    pub data_dir: String,
    /// The embedding provider, if configured, and its usage metrics.
    pub embeddings: EmbeddingsManager,
    /// The opened device and its memory budget, under the gpu profile.
    pub gpu: Option<Arc<GpuManager>>,
    /// The loaded model, when runtime.inference.enabled is set.
    pub inference: Option<Arc<InferenceManager>>,
    /// When the model was loaded, in seconds since the Unix epoch.
    pub inference_loaded_at: u64,
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
    /// The inference settings the model was loaded with. A reload that changes them is refused.
    booted_inference: InferenceConfig,
    /// Where a reload reads configuration from.
    config_source: ConfigSource,
    /// Time of startup or of the last successful reload, in seconds since the Unix epoch.
    pub config_last_reload: Arc<AtomicU64>,
}

impl AppState {
    /// Build the state from a loaded configuration, creating the data directory if missing.
    pub fn new(config: Config, embeddings: EmbeddingsManager) -> Result<Self> {
        let data_dir = config.startup.data_dir.clone();
        std::fs::create_dir_all(&data_dir)?;
        if config.startup.disk.min_free_bytes.is_some()
            && super::disk::free_bytes(&data_dir)?.is_none()
        {
            return Err(ServerError::InvalidRequest(
                "startup.disk.min_free_bytes is set, and this platform cannot measure free disk space"
                    .into(),
            )
            .into());
        }
        let booted_with = config.startup.clone();
        let booted_inference = config.runtime.inference.clone();
        let app_config = Arc::new(RwLock::new(config));

        Ok(Self {
            collection_manager: CollectionManager::new(data_dir.clone(), app_config.clone()),
            data_dir,
            embeddings,
            gpu: None,
            inference: None,
            inference_loaded_at: 0,
            machine: MachineReadings::start(SAMPLE_INTERVAL)?,
            shutting_down: Arc::new(AtomicBool::new(false)),
            read_only: Arc::new(AtomicBool::new(false)),
            app_config,
            booted_with,
            booted_inference,
            config_source: ConfigSource::default(),
            config_last_reload: Arc::new(AtomicU64::new(piramid_core::clock::unix_secs()?)),
        })
    }

    /// Account device memory against an opened GPU.
    #[must_use]
    pub fn with_gpu(mut self, manager: Arc<GpuManager>) -> Self {
        self.gpu = Some(manager);
        self
    }

    /// Serve generations from a loaded model. Errors when the clock reads before 1970.
    pub fn with_inference(mut self, manager: Arc<InferenceManager>) -> Result<Self> {
        self.inference = Some(manager);
        self.inference_loaded_at = piramid_core::clock::unix_secs()?;
        Ok(self)
    }

    /// Read reloads from source, the one the process booted from.
    #[must_use]
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

    /// Handle to a collection that is loaded or present on disk, opening it if needed.
    pub fn get_existing_collection(&self, name: &str) -> Result<CollectionHandle> {
        self.ensure_available()?;
        self.collection_manager.get_existing(name)
    }

    /// Handle to a collection, opening or creating it if needed.
    pub fn get_or_create_collection(&self, name: &str) -> Result<CollectionHandle> {
        self.ensure_available()?;
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
        if new_cfg.runtime.inference != self.booted_inference {
            return Err(ServerError::InvalidRequest(
                "runtime.inference changed; the model is loaded with it at boot, so this needs a restart"
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
        let now = piramid_core::clock::unix_secs()?;
        self.config_last_reload.store(now, Ordering::Relaxed);
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

    /// Error with 503 when shutting down or below the free-space floor. With read-only on low
    /// space enabled the server stays read-only until the first write that finds the space back.
    pub fn ensure_write_allowed(&self) -> Result<()> {
        self.ensure_available()?;
        let Some(min_free) = self.disk_min_free_bytes() else {
            return Ok(());
        };
        let free = self.disk_free_bytes()?.ok_or_else(|| {
            ServerError::Internal(
                "startup.disk.min_free_bytes is set, and this platform cannot measure free disk \
                 space"
                    .into(),
            )
        })?;
        if free >= min_free {
            if self.read_only.swap(false, Ordering::Relaxed) {
                tracing::info!(
                    target: "piramid::disk",
                    free_bytes = free,
                    min_free = min_free,
                    "write_access_restored"
                );
            }
            return Ok(());
        }
        if !self.disk_readonly_on_low_space() {
            tracing::warn!(
                target: "piramid::disk",
                free_bytes = free,
                min_free = min_free,
                "disk_space_low"
            );
            return Err(ServerError::ServiceUnavailable(format!(
                "free disk space {free} bytes is below startup.disk.min_free_bytes {min_free}"
            ))
            .into());
        }
        self.read_only.store(true, Ordering::Relaxed);
        Err(
            ServerError::ServiceUnavailable("Low disk space; write operations disabled".into())
                .into(),
        )
    }
}

/// Reference-counted handle to [AppState].
pub type SharedState = Arc<AppState>;
