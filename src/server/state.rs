use std::sync::{Arc, RwLock, atomic::{AtomicBool, Ordering}};
use dashmap::DashMap;

use crate::Collection;
use crate::embeddings::Embedder;
use crate::metrics::LatencyTracker;
use crate::error::{Result, ServerError};

// Shared application state
// Each collection is an independent Collection with its own file.
// DashMap allows concurrent access to different collections without blocking
pub struct AppState {
    pub collections: DashMap<String, Arc<RwLock<Collection>>>,
    pub data_dir: String,
    pub embedder: Option<Arc<dyn Embedder>>,
    pub shutting_down: Arc<AtomicBool>,
    pub latency_tracker: Arc<DashMap<String, LatencyTracker>>,  // Per-collection latency tracking
}

impl AppState {
    pub fn new(data_dir: &str) -> Self {
        std::fs::create_dir_all(data_dir).ok();
        
        Self {
            collections: DashMap::new(),
            data_dir: data_dir.to_string(),
            embedder: None,
            shutting_down: Arc::new(AtomicBool::new(false)),
            latency_tracker: Arc::new(DashMap::new()),
        }
    }

    pub fn with_embedder(data_dir: &str, embedder: Arc<dyn Embedder>) -> Self {
        std::fs::create_dir_all(data_dir).ok();
        
        Self {
            collections: DashMap::new(),
            data_dir: data_dir.to_string(),
            embedder: Some(embedder),
            shutting_down: Arc::new(AtomicBool::new(false)),
            latency_tracker: Arc::new(DashMap::new()),
        }
    }

    // Lazily load or create a collection
    pub fn get_or_create_collection(&self, name: &str) -> Result<()> {
        if self.shutting_down.load(Ordering::Relaxed) {
            return Err(ServerError::ServiceUnavailable("Server is shutting down".into()).into());
        }

        if !self.collections.contains_key(name) {
            let path = format!("{}/{}.db", self.data_dir, name);
            let storage = Collection::open(&path)?;
            self.collections.insert(name.to_string(), Arc::new(RwLock::new(storage)));
            
            // Create latency tracker for this collection
            self.latency_tracker.insert(name.to_string(), LatencyTracker::new());
        }
        
        Ok(())
    }

    pub fn checkpoint_all(&self) -> Result<()> {
        for mut entry in self.collections.iter_mut() {
            let storage = entry.value_mut();
            let mut storage_guard = storage.write()
                .map_err(|e| ServerError::Internal(format!("Lock poisoned: {}", e)))?;
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
