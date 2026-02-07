use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::VectorStorage;
use crate::embeddings::Embedder;
use crate::error::{Result, ServerError};

// Shared application state
// Each collection is an independent VectorStorage with its own file.
// RwLock allows concurrent reads but exclusive writes.
pub struct AppState {
    pub collections: RwLock<HashMap<String, VectorStorage>>,
    pub data_dir: String,
    pub embedder: Option<Arc<dyn Embedder>>,
}

impl AppState {
    pub fn new(data_dir: &str) -> Self {
        std::fs::create_dir_all(data_dir).ok();
        
        Self {
            collections: RwLock::new(HashMap::new()),
            data_dir: data_dir.to_string(),
            embedder: None,
        }
    }

    pub fn with_embedder(data_dir: &str, embedder: Arc<dyn Embedder>) -> Self {
        std::fs::create_dir_all(data_dir).ok();
        
        Self {
            collections: RwLock::new(HashMap::new()),
            data_dir: data_dir.to_string(),
            embedder: Some(embedder),
        }
    }

    // Lazily load or create a collection
    pub fn get_or_create_collection(&self, name: &str) -> Result<()> {
        let mut collections = self.collections.write()
            .map_err(|e| ServerError::Internal(format!("Lock poisoned: {}", e)))?;
        
        if !collections.contains_key(name) {
            let path = format!("{}/{}.db", self.data_dir, name);
            let storage = VectorStorage::open(&path)?;
            collections.insert(name.to_string(), storage);
        }
        
        Ok(())
    }

    pub fn checkpoint_all(&self) -> Result<()> {
        let mut collections = self.collections.write()
            .map_err(|e| ServerError::Internal(format!("Lock poisoned: {}", e)))?;
        for storage in collections.values_mut() {
            storage.checkpoint()?;
            storage.flush()?;
        }
        Ok(())
    }

}

pub type SharedState = Arc<AppState>;
