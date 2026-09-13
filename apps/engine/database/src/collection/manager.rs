//! The registry of open collections under one data directory.

use dashmap::{mapref::one::Ref, DashMap};
use parking_lot::RwLock;
use std::sync::Arc;
use tokio::runtime::Handle;

use crate::storage::SidecarManager;
use crate::Collection;
use crate::CollectionOpenOptions;
use piramid_core::config::Config;
use piramid_core::error::{Result, ServerError};
use piramid_core::stats::LatencyTracker;

/// A shared, lockable reference to an open collection.
pub type CollectionHandle = Arc<RwLock<Collection>>;

/// Open collections by name, each with its latency tracker, under one data directory.
pub struct CollectionManager {
    collections: DashMap<String, CollectionHandle>,
    latency_trackers: DashMap<String, LatencyTracker>,
    data_dir: String,
    app_config: Arc<RwLock<Config>>,
}

impl CollectionManager {
    /// A manager with nothing open, reading collection defaults from app_config.
    pub fn new(data_dir: String, app_config: Arc<RwLock<Config>>) -> Self {
        Self {
            collections: DashMap::new(),
            latency_trackers: DashMap::new(),
            data_dir,
            app_config,
        }
    }

    /// The named collection, opening it from disk if needed. Fails when no data file exists.
    pub fn get_existing(&self, name: &str) -> Result<CollectionHandle> {
        piramid_core::validation::validate_collection_name(name)?;
        if let Some(existing) = self.collections.get(name) {
            return Ok(existing.value().clone());
        }

        let path = self.collection_path(name);
        if !std::path::Path::new(&path).exists() {
            return Err(ServerError::NotFound("Collection not found".into()).into());
        }

        self.open_and_register(name, &path)
    }

    /// The named collection, opening or creating its data file if needed.
    pub fn get_or_create(&self, name: &str) -> Result<CollectionHandle> {
        piramid_core::validation::validate_collection_name(name)?;
        if let Some(existing) = self.collections.get(name) {
            return Ok(existing.value().clone());
        }

        let path = self.collection_path(name);
        self.open_and_register(name, &path)
    }

    fn open_and_register(&self, name: &str, path: &str) -> Result<CollectionHandle> {
        let cfg = { self.app_config.read().clone() };
        let collection = Collection::open_with_options(
            path,
            CollectionOpenOptions::from(cfg.to_collection_config()),
        )?;
        let handle = Arc::new(RwLock::new(collection));

        self.collections.insert(name.to_string(), handle.clone());
        self.latency_trackers
            .insert(name.to_string(), LatencyTracker::new());
        self.warm_page_cache(handle.clone());

        Ok(handle)
    }

    /// Closes a collection if it is open and deletes its data file and sidecars.
    pub fn delete(&self, name: &str) -> Result<()> {
        piramid_core::validation::validate_collection_name(name)?;
        self.latency_trackers.remove(name);
        let was_open = self.collections.remove(name).is_some();
        let base = self.collection_path(name);
        let mut removed_any = false;
        for path in std::iter::once(base.clone()).chain(SidecarManager::at(&base).all_paths()) {
            match std::fs::remove_file(&path) {
                Ok(()) => removed_any = true,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }
        }
        if !was_open && !removed_any {
            return Err(ServerError::NotFound(format!("collection '{name}' not found")).into());
        }
        Ok(())
    }

    /// Whether the named collection is open.
    pub fn contains_loaded(&self, name: &str) -> bool {
        self.collections.contains_key(name)
    }

    /// Number of open collections.
    pub fn len(&self) -> usize {
        self.collections.len()
    }

    /// Whether no collection is open.
    pub fn is_empty(&self) -> bool {
        self.collections.is_empty()
    }

    /// Name and handle of every open collection.
    pub fn loaded_collections(&self) -> Vec<(String, CollectionHandle)> {
        self.collections
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().clone()))
            .collect()
    }

    /// Latency tracker of the named open collection.
    pub fn tracker(&self, name: &str) -> Option<Ref<'_, String, LatencyTracker>> {
        self.latency_trackers.get(name)
    }

    fn collection_path(&self, name: &str) -> String {
        record_path(&self.data_dir, name)
    }

    /// Warms in the background on the current tokio runtime, and does nothing without one.
    fn warm_page_cache(&self, handle: CollectionHandle) {
        let Ok(runtime) = Handle::try_current() else {
            return;
        };
        runtime.spawn_blocking(move || {
            let guard = handle.read();
            guard.warm_page_cache();
        });
    }
}

/// Collection names present in the data directory, read from file names without opening any.
pub fn collection_names(data_dir: &str) -> Result<Vec<String>> {
    let mut names = Vec::new();
    for entry in std::fs::read_dir(data_dir)? {
        let entry = entry?;
        if let Some(name) = entry.file_name().to_str().and_then(collection_name_of) {
            names.push(name);
        }
    }
    names.sort();
    names.dedup();
    Ok(names)
}

/// Path of the record file of the named collection in the data directory.
pub fn record_path(data_dir: &str, name: &str) -> String {
    format!("{data_dir}/{name}.db")
}

/// The collection a data file belongs to, or None if it is a sidecar or unrelated.
fn collection_name_of(file_name: &str) -> Option<String> {
    if SidecarManager::SUFFIXES
        .iter()
        .any(|suffix| file_name.ends_with(suffix))
    {
        return None;
    }
    let name = file_name.strip_suffix(".db")?;
    (!name.is_empty()).then(|| name.to_string())
}
