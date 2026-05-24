use dashmap::{mapref::one::Ref, DashMap};
use parking_lot::RwLock;
use std::sync::Arc;
use tokio::runtime::Handle;

use crate::collections::CollectionOpenOptions;
use crate::config::AppConfig;
use crate::error::{Result, ServerError};
use crate::metrics::LatencyTracker;
use crate::Collection;

pub type CollectionHandle = Arc<RwLock<Collection>>;

pub struct CollectionManager {
    collections: DashMap<String, CollectionHandle>,
    latency_trackers: DashMap<String, LatencyTracker>,
    data_dir: String,
    app_config: Arc<RwLock<AppConfig>>,
}

impl CollectionManager {
    pub fn new(data_dir: String, app_config: Arc<RwLock<AppConfig>>) -> Self {
        Self {
            collections: DashMap::new(),
            latency_trackers: DashMap::new(),
            data_dir,
            app_config,
        }
    }

    pub fn get_existing(&self, name: &str) -> Result<CollectionHandle> {
        if let Some(existing) = self.collections.get(name) {
            return Ok(existing.value().clone());
        }

        let path = self.collection_path(name);
        if !std::path::Path::new(&path).exists() {
            return Err(ServerError::NotFound("Collection not found".into()).into());
        }

        self.open_and_register(name, &path)
    }

    pub fn get_or_create(&self, name: &str) -> Result<CollectionHandle> {
        if let Some(existing) = self.collections.get(name) {
            return Ok(existing.value().clone());
        }

        let path = self.collection_path(name);
        self.open_and_register(name, &path)
    }

    fn open_and_register(&self, name: &str, path: &str) -> Result<CollectionHandle> {
        let cfg = { self.app_config.read().clone() };
        let storage = Collection::open_with_options(
            path,
            CollectionOpenOptions::from(cfg.to_collection_config()),
        )?;
        let handle = Arc::new(RwLock::new(storage));

        self.collections.insert(name.to_string(), handle.clone());
        self.latency_trackers
            .insert(name.to_string(), LatencyTracker::new());
        self.warm_page_cache(handle.clone());

        Ok(handle)
    }

    pub fn remove(&self, name: &str) -> Option<CollectionHandle> {
        self.latency_trackers.remove(name);
        self.collections.remove(name).map(|(_, handle)| handle)
    }

    pub fn contains_loaded(&self, name: &str) -> bool {
        self.collections.contains_key(name)
    }

    pub fn len(&self) -> usize {
        self.collections.len()
    }

    pub fn is_empty(&self) -> bool {
        self.collections.is_empty()
    }

    pub fn loaded_collections(&self) -> Vec<(String, CollectionHandle)> {
        self.collections
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().clone()))
            .collect()
    }

    pub fn tracker(&self, name: &str) -> Option<Ref<'_, String, LatencyTracker>> {
        self.latency_trackers.get(name)
    }

    fn collection_path(&self, name: &str) -> String {
        format!("{}/{}.db", self.data_dir, name)
    }

    fn warm_page_cache(&self, handle: CollectionHandle) {
        if let Ok(rt) = Handle::try_current() {
            rt.spawn_blocking(move || {
                let guard = handle.read();
                guard.warm_page_cache();
            });
        }
    }
}

#[deprecated(note = "use CollectionManager")]
pub type CollectionRegistry = CollectionManager;
