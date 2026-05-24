use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::{Result, ServerError};
use crate::metrics::record_lock_read;
use crate::runtime::SharedState;
use crate::server::types::*;

fn ensure_available(state: &SharedState) -> Result<()> {
    if state
        .shutting_down
        .load(std::sync::atomic::Ordering::Relaxed)
    {
        return Err(ServerError::ServiceUnavailable("Server is shutting down".to_string()).into());
    }
    Ok(())
}

pub fn health() -> HealthResponse {
    HealthResponse {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
    }
}

pub fn embeddings_available(state: &SharedState) -> bool {
    state.embedder.is_some()
}

pub fn config_status(state: &SharedState) -> Result<ConfigStatusResponse> {
    ensure_available(state)?;
    Ok(ConfigStatusResponse {
        app_config: state.current_config(),
        reloaded_at: Some(
            state
                .config_last_reload
                .load(std::sync::atomic::Ordering::Relaxed),
        ),
    })
}

pub fn reload_config(state: &SharedState) -> Result<ConfigReloadResponse> {
    ensure_available(state)?;
    let app_config = state.reload_config()?;
    Ok(ConfigReloadResponse {
        success: true,
        reloaded_at: Some(
            state
                .config_last_reload
                .load(std::sync::atomic::Ordering::Relaxed),
        ),
        app_config,
    })
}

pub fn metrics(state: &SharedState) -> Result<MetricsResponse> {
    let mut collection_metrics = Vec::new();
    let mut wal_stats = Vec::new();
    let mut total_vectors = 0;

    for (collection_name, collection_handle) in state.collection_manager.loaded_collections() {
        let lock_start = std::time::Instant::now();
        let collection_guard = collection_handle.read();
        record_lock_read(
            state
                .collection_manager
                .tracker(&collection_name)
                .as_deref(),
            lock_start,
        );
        let count = collection_guard.count();
        let index_type = collection_guard.vector_index().index_type().to_string();
        let memory_usage_bytes = collection_guard.memory_usage_bytes();
        let (insert_latency_ms, search_latency_ms, lock_read_ms, lock_write_ms) =
            if let Some(tracker) = state.collection_manager.tracker(&collection_name) {
                (
                    tracker.avg_insert_latency_ms(),
                    tracker.avg_search_latency_ms(),
                    tracker.avg_lock_read_latency_ms(),
                    tracker.avg_lock_write_latency_ms(),
                )
            } else {
                (None, None, None, None)
            };

        total_vectors += count;
        let (search_overfetch, hnsw_ef_search, ivf_nprobe) = match &collection_guard.config.index {
            crate::index::IndexConfig::Auto { search, .. } => {
                (Some(search.filter_overfetch), None, None)
            }
            crate::index::IndexConfig::Flat { search, .. } => {
                (Some(search.filter_overfetch), None, None)
            }
            crate::index::IndexConfig::Hnsw {
                ef_search, search, ..
            } => (Some(search.filter_overfetch), Some(*ef_search), None),
            crate::index::IndexConfig::Ivf {
                num_probes, search, ..
            } => (Some(search.filter_overfetch), None, Some(*num_probes)),
        };

        collection_metrics.push(CollectionMetrics {
            name: collection_name,
            vector_count: count,
            index_type,
            memory_usage_bytes,
            insert_latency_ms,
            search_latency_ms,
            lock_read_ms,
            lock_write_ms,
            search_overfetch,
            hnsw_ef_search,
            ivf_nprobe,
        });

        let wal_size = std::fs::metadata(format!("{}.wal.db", collection_guard.path))
            .map(|metadata| metadata.len())
            .ok();
        let checkpoint_age_secs =
            collection_guard
                .checkpoint
                .last_checkpoint()
                .and_then(|timestamp| {
                    let now = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs();
                    now.checked_sub(timestamp)
                });
        wal_stats.push(WalStats {
            collection: collection_guard.path.clone(),
            last_checkpoint: collection_guard.checkpoint.last_checkpoint(),
            checkpoint_age_secs,
            wal_size_bytes: wal_size,
        });
    }

    let embed_metrics = state.embed_metrics.snapshot();
    Ok(MetricsResponse {
        total_collections: state.collection_manager.len(),
        total_vectors,
        collections: collection_metrics,
        app_config: state.current_config(),
        wal_stats,
        embedding: EmbeddingMetricsResponse {
            requests: embed_metrics.requests,
            texts: embed_metrics.texts,
            total_tokens: embed_metrics.total_tokens,
            avg_latency_ms: embed_metrics.avg_latency_ms,
        },
    })
}

pub fn readyz(state: &SharedState) -> Result<ReadyzResponse> {
    ensure_available(state)?;

    let mut collections = Vec::new();
    let mut total_vectors = 0usize;

    for (name, collection_handle) in state.collection_manager.loaded_collections() {
        let lock_start = std::time::Instant::now();
        let collection_guard = collection_handle.read();
        record_lock_read(
            state.collection_manager.tracker(&name).as_deref(),
            lock_start,
        );

        let count = collection_guard.count();
        total_vectors += count;
        let last_checkpoint = collection_guard.checkpoint.last_checkpoint();
        let checkpoint_age_secs = last_checkpoint.and_then(|timestamp| {
            let now = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs();
            now.checked_sub(timestamp)
        });
        let wal_size_bytes = std::fs::metadata(format!("{}.wal.db", collection_guard.path))
            .map(|metadata| metadata.len())
            .ok();

        collections.push(CollectionHealth {
            name,
            loaded: true,
            count: Some(count),
            index_type: Some(collection_guard.vector_index().index_type().to_string()),
            last_checkpoint,
            checkpoint_age_secs,
            wal_size_bytes,
            schema_version: Some(collection_guard.metadata.schema_version),
            integrity_ok: true,
            error: None,
        });
    }

    if let Ok(entries) = std::fs::read_dir(&state.data_dir) {
        for entry in entries.flatten() {
            if entry.path().extension().is_some_and(|ext| ext == "db") {
                let name = entry
                    .path()
                    .file_stem()
                    .and_then(|stem| stem.to_str())
                    .unwrap_or("")
                    .to_string();
                if state.collection_manager.contains_loaded(&name) {
                    continue;
                }
                collections.push(CollectionHealth {
                    name,
                    loaded: false,
                    count: None,
                    index_type: None,
                    last_checkpoint: None,
                    checkpoint_age_secs: None,
                    wal_size_bytes: None,
                    schema_version: None,
                    integrity_ok: false,
                    error: Some("not loaded".to_string()),
                });
            }
        }
    }

    let loaded_collections = state.collection_manager.len();
    let (disk_total_bytes, disk_available_bytes) = disk_stats(&state.data_dir);
    let ok = collections
        .iter()
        .all(|collection| collection.integrity_ok && collection.loaded);

    Ok(ReadyzResponse {
        ok,
        version: env!("CARGO_PKG_VERSION").to_string(),
        data_dir: state.data_dir.clone(),
        total_collections: collections.len(),
        loaded_collections,
        total_vectors,
        disk_total_bytes,
        disk_available_bytes,
        collections,
    })
}

fn disk_stats(path: &str) -> (Option<u64>, Option<u64>) {
    #[cfg(target_family = "unix")]
    {
        use std::ffi::CString;
        let path = match CString::new(path) {
            Ok(path) => path,
            Err(_) => return (None, None),
        };
        let mut stat: libc::statvfs = unsafe { std::mem::zeroed() };
        let rc = unsafe { libc::statvfs(path.as_ptr(), &mut stat) };
        if rc == 0 {
            let total = (stat.f_blocks as u64).saturating_mul(stat.f_frsize as u64);
            let available = (stat.f_bavail as u64).saturating_mul(stat.f_frsize as u64);
            return (Some(total), Some(available));
        }
    }
    (None, None)
}
