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

// Shared application state
// Each collection is an independent Collection with its own file.
// DashMap allows concurrent access to different collections without blocking.
// Holds config + optional embedder so handlers can access without reloading.
pub struct AppState {
    pub collections: DashMap<String, Arc<RwLock<Collection>>>,
    pub data_dir: String,
    pub embedder: Option<Arc<dyn Embedder>>,
    pub shutting_down: Arc<AtomicBool>,
    pub latency_tracker: Arc<DashMap<String, LatencyTracker>>,  // Per-collection latency tracking
    pub app_config: AppConfig,
    pub slow_query_ms: u128,
}

impl AppState {
    pub fn new(data_dir: &str, app_config: AppConfig, slow_query_ms: u128) -> Self {
        std::fs::create_dir_all(data_dir).ok();
        
        Self {
            collections: DashMap::new(),
            data_dir: data_dir.to_string(),
            embedder: None,
            shutting_down: Arc::new(AtomicBool::new(false)),
            latency_tracker: Arc::new(DashMap::new()),
            app_config,
            slow_query_ms,
        }
    }

    pub fn with_embedder(data_dir: &str, app_config: AppConfig, slow_query_ms: u128, embedder: Arc<dyn Embedder>) -> Self {
        std::fs::create_dir_all(data_dir).ok();
        
        Self {
            collections: DashMap::new(),
            data_dir: data_dir.to_string(),
            embedder: Some(embedder),
            shutting_down: Arc::new(AtomicBool::new(false)),
            latency_tracker: Arc::new(DashMap::new()),
            app_config,
            slow_query_ms,
        }
    }

    // Lazily load or create a collection
    pub fn get_or_create_collection(&self, name: &str) -> Result<()> {

        

        if self.shutting_down.load(Ordering::Relaxed) {
            return Err(ServerError::ServiceUnavailable("Server is shutting down".into()).into());
        }

        if !self.collections.contains_key(name) {
            let path = format!("{}/{}.db", self.data_dir, name);
            let storage = Collection::open_with_options(
                &path,
                CollectionOpenOptions::from(self.app_config.to_collection_config()),
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
    
    pub fn initiate_shutdown(&self) {
        self.shutting_down.store(true, Ordering::Relaxed);
    }
}

pub type SharedState = Arc<AppState>;
