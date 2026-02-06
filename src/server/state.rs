use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::VectorStorage;
use crate::embeddings::Embedder;

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
    pub fn get_or_create_collection(&self, name: &str) -> Result<(), String> {
        let mut collections = self.collections.write()
            .map_err(|e| format!("Lock poisoned: {}", e))?;
        
        if !collections.contains_key(name) {
            let path = format!("{}/{}.db", self.data_dir, name);
            let storage = VectorStorage::open(&path)
                .map_err(|e| format!("Failed to open collection: {}", e))?;
            collections.insert(name.to_string(), storage);
        }
        
        Ok(())
    }
}

pub type SharedState = Arc<AppState>;
