//! In a web server, you need to share data across requests.
//! But HTTP is stateless - each request is independent.
//! 
//! Solution: wrap shared data in `Arc<T>` (atomic reference counting)
//! so multiple threads can share ownership safely.
//!
//! For mutable shared data, we also need `RwLock` or `Mutex`:
//! - `Mutex` = one reader OR one writer at a time
//! - `RwLock` = many readers OR one writer (better for read-heavy workloads)

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::VectorStorage;
use crate::embeddings::Embedder;

/// The shared state that lives for the lifetime of the server.
/// 
/// Each collection is its own `VectorStorage` instance with its own file.
/// The `RwLock` lets us add/remove collections while others read.
pub struct AppState {
    /// name → storage mapping. RwLock for concurrent access.
    pub collections: RwLock<HashMap<String, VectorStorage>>,
    /// where .db files live (e.g., "./piramid_data")
    pub data_dir: String,
    /// Optional embedder for generating vectors from text
    pub embedder: Option<Arc<dyn Embedder>>,
}

impl AppState {
    /// Create new app state, ensuring data directory exists
    pub fn new(data_dir: &str) -> Self {
        // ok() discards the Result - we don't care if dir already exists
        std::fs::create_dir_all(data_dir).ok();
        
        Self {
            collections: RwLock::new(HashMap::new()),
            data_dir: data_dir.to_string(),
            embedder: None,
        }
    }

    /// Create app state with an embedder
    pub fn with_embedder(data_dir: &str, embedder: Arc<dyn Embedder>) -> Self {
        std::fs::create_dir_all(data_dir).ok();
        
        Self {
            collections: RwLock::new(HashMap::new()),
            data_dir: data_dir.to_string(),
            embedder: Some(embedder),
        }
    }

    /// Load or create a collection. Called on first access.
    /// 
    /// This is lazy loading - we don't load all collections on startup,
    /// just when someone first requests them.
    pub fn get_or_create_collection(&self, name: &str) -> Result<(), String> {
        // write() gives us exclusive access (we might insert)
        let mut collections = self.collections.write().unwrap();
        
        if !collections.contains_key(name) {
            let path = format!("{}/{}.db", self.data_dir, name);
            let storage = VectorStorage::open(&path)
                .map_err(|e| format!("Failed to open collection: {}", e))?;
            collections.insert(name.to_string(), storage);
        }
        
        Ok(())
    }
}

/// Type alias: Arc<AppState> is verbose, SharedState is nicer
/// 
/// Arc = "Atomically Reference Counted"
/// It's like a shared_ptr in C++ - multiple owners, freed when last one drops.
/// Arc is thread-safe (Rc is the single-threaded version).
pub type SharedState = Arc<AppState>;
