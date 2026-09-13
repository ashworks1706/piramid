//! Opening a collection: load sidecars, replay the WAL, rebuild what is missing.

/// Settings a collection is opened with.
#[derive(Clone, Default)]
pub struct CollectionOpenOptions {
    /// Configuration the collection runs with.
    pub config: piramid_core::config::CollectionConfig,
}

impl From<piramid_core::config::CollectionConfig> for CollectionOpenOptions {
    fn from(config: piramid_core::config::CollectionConfig) -> Self {
        Self { config }
    }
}

use std::collections::HashMap;
use uuid::Uuid;

use super::checkpoint::CheckpointManager;
use super::Collection;
use crate::cache::CacheManager;
use crate::index::HashMapVectorReader;
use crate::index::{load_vector_index, save_vector_index, VectorIndex};
use crate::storage::manifest::CollectionMetadata;
use crate::storage::record_store::RecordStore;
use crate::storage::wal::{Wal, WalEntry};
use crate::storage::SidecarManager;
use piramid_core::config::{IndexConfig, IndexKind};
use piramid_core::error::{IndexError, Result, StorageError};
use piramid_core::Document;
use piramid_hardware::compute::ExecutionMode;

/// Open the collection at path, replaying the WAL and rebuilding sidecars as needed.
pub fn open(path: &str, options: CollectionOpenOptions) -> Result<Collection> {
    let config = options.config;

    // A path with no usable file stem is an error.
    let collection_name = std::path::Path::new(path)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .ok_or_else(|| {
            StorageError::InvalidPath(format!(
                "collection path '{path}' has no usable file stem to name the collection"
            ))
        })?
        .to_string();

    let sidecars = SidecarManager::at(path);
    let index = sidecars.load_offsets()?;
    let record_store = RecordStore::open(path, &config, &index)?;

    let manifest = match sidecars.load_manifest()? {
        Some(meta) => {
            let mut meta = meta;
            meta.update_vector_count(index.len())?;
            meta
        }
        None if index.is_empty() => CollectionMetadata::new(collection_name.clone())?,
        None => {
            return Err(StorageError::CorruptedData(format!(
                "collection '{collection_name}' has {} documents but no manifest",
                index.len()
            ))
            .into())
        }
    };

    let loaded_vector_index = load_vector_index(path)?;
    let mut rebuild_required = loaded_vector_index.is_none();
    let mut vector_index = match loaded_vector_index {
        Some(mut loaded) => {
            let configured = config.index.metric();
            if loaded.metric() != configured {
                return Err(IndexError::InvalidConfig(format!(
                    "collection '{collection_name}' is indexed by {} but the configuration asks \
                     for {}; rebuild the collection to change its metric",
                    loaded.metric().as_str(),
                    configured.as_str()
                ))
                .into());
            }
            match family_to_rebuild(loaded.as_ref(), &config.index, index.len()) {
                None => {
                    loaded.set_execution(config.execution);
                    loaded
                }
                Some(kind) => {
                    tracing::info!(
                        target: "piramid::indexing",
                        collection = collection_name.as_str(),
                        from = %loaded.index_type(),
                        to = ?kind,
                        "index_rebuilt_for_config"
                    );
                    rebuild_required = true;
                    crate::index::create_index_of_kind(
                        &config.index,
                        kind,
                        config.execution,
                        index.len(),
                    )
                }
            }
        }
        None => crate::index::create_index(&config.index, config.execution, index.len()),
    };

    let min_seq = if config.wal.enabled {
        sidecars.load_wal_meta()?
    } else {
        0
    };
    let next_seq = min_seq + 1;

    let wal_path = sidecars.wal_path();

    let wal = if config.wal.enabled {
        Wal::new(wal_path.into(), next_seq, config.wal.sync_on_write)?
    } else {
        Wal::disabled(wal_path.into(), next_seq)
    };

    let checkpoint = CheckpointManager::new(wal)?;

    let wal_entries = if config.wal.enabled {
        checkpoint.wal.replay(min_seq)?
    } else {
        Vec::new()
    };

    // A missing ANN sidecar, or one built with other settings than config.index, is rebuilt from
    // the record store before the WAL replays on top of it.
    if rebuild_required && !index.is_empty() {
        rebuild_vector_index(&mut vector_index, &index, &record_store)?;
        save_vector_index(path, vector_index.as_ref())?;
    }

    let mut collection = Collection {
        record_store,
        index,
        vector_index,
        cache: CacheManager::new(config.cache),
        config,
        manifest,
        path: path.to_string(),
        checkpoint,
    };

    // The stored vectors are resident before replay, so replayed inserts can link to them.
    collection.rebuild_vector_cache()?;

    // The WAL is cleared only once the checkpoint has made the replayed state durable.
    if !wal_entries.is_empty() {
        replay_wal(&mut collection, wal_entries)?;
        super::checkpoint::checkpoint(&mut collection)?;
    }
    Ok(collection)
}

fn replay_wal(collection: &mut Collection, entries: Vec<WalEntry>) -> Result<()> {
    for entry in entries {
        match entry {
            WalEntry::Insert {
                id,
                vector,
                text,
                metadata,
                ..
            } => {
                let document = Document {
                    id,
                    vector,
                    text,
                    metadata,
                };
                crate::document::insert_internal(collection, document)?;
            }
            // An update replays as a delete followed by an insert.
            WalEntry::Update {
                id,
                vector,
                text,
                metadata,
                ..
            } => {
                crate::document::delete_internal(collection, &id)?;
                let document = Document {
                    id,
                    vector,
                    text,
                    metadata,
                };
                crate::document::insert_internal(collection, document)?;
            }
            WalEntry::Delete { id, .. } => {
                crate::document::delete_internal(collection, &id)?;
            }
            WalEntry::Checkpoint { .. } => {}
        }
    }
    Ok(())
}

/// The family to rebuild into when loaded was built with other settings than config asks for at
/// num_vectors, or None when loaded matches.
///
/// An auto config keeps a loaded family ranked above the one its size picks. The IVF partition
/// and probe counts an auto config leaves unset are sized from the collection and not compared.
fn family_to_rebuild(
    loaded: &dyn VectorIndex,
    config: &IndexConfig,
    num_vectors: usize,
) -> Option<IndexKind> {
    use crate::index::{growth_rank, kind_of};

    let loaded_kind = kind_of(loaded.index_type());
    let selected = config.select_type(num_vectors);
    let wanted_kind = match config {
        IndexConfig::Auto { .. } if growth_rank(loaded_kind) > growth_rank(selected) => loaded_kind,
        _ => selected,
    };
    let built = loaded.build_config();
    let wanted = crate::index::create_index_of_kind(
        config,
        wanted_kind,
        ExecutionMode::default(),
        num_vectors,
    )
    .build_config();
    let matches = match (config, &built, &wanted) {
        (
            IndexConfig::Auto { auto, .. },
            IndexConfig::Ivf { params: built },
            IndexConfig::Ivf { params: wanted },
        ) => {
            let clusters_sized = auto.ivf_num_clusters.is_none();
            let probes_sized = clusters_sized && auto.ivf_num_probes.is_none();
            built.metric == wanted.metric
                && built.max_iterations == wanted.max_iterations
                && (clusters_sized || built.num_clusters == wanted.num_clusters)
                && (probes_sized || built.num_probes == wanted.num_probes)
        }
        _ => built == wanted,
    };
    (!matches).then_some(wanted_kind)
}

fn rebuild_vector_index(
    vector_index: &mut Box<dyn VectorIndex>,
    index: &HashMap<Uuid, crate::storage::sidecars::EntryPointer>,
    record_store: &RecordStore,
) -> Result<()> {
    let mut vectors: HashMap<Uuid, Vec<f32>> = HashMap::new();
    for (id, idx_entry) in index {
        let entry = record_store.read_document(idx_entry)?;
        vectors.insert(*id, entry.vector().to_vec());
    }

    let reader = HashMapVectorReader::new(&vectors);
    for (id, vector) in &vectors {
        vector_index.insert(*id, vector, &reader)?;
    }
    Ok(())
}
