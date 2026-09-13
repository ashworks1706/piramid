//! Health, readiness, metrics and configuration operations.

use crate::services::api::*;
use crate::state::SharedState;
use piramid_core::error::Result;
use piramid_core::stats::record_lock_read;
use piramid_database::storage::SidecarManager;

/// Liveness of the process and the binary version.
pub fn health() -> HealthResponse {
    HealthResponse {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
    }
}

/// Whether an embedding provider is configured.
pub fn embeddings_available(state: &SharedState) -> bool {
    state.embeddings.is_configured()
}

/// The configuration in effect and the time it was last loaded.
pub fn config_status(state: &SharedState) -> Result<ConfigStatusResponse> {
    state.ensure_available()?;
    Ok(ConfigStatusResponse {
        app_config: state.current_config(),
        reloaded_at: Some(
            state
                .config_last_reload
                .load(std::sync::atomic::Ordering::Relaxed),
        ),
    })
}

/// Reload configuration from disk and environment and return the result.
pub fn reload_config(state: &SharedState) -> Result<ConfigReloadResponse> {
    state.ensure_available()?;
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

/// Metrics snapshot of every loaded collection, the configuration and embedding usage.
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
        let memory_usage_bytes = collection_guard.memory_usage_bytes()?;
        let (insert_latency_ms, search_latency_ms, lock_read_ms, lock_write_ms) = state
            .collection_manager
            .tracker(&collection_name)
            .map(|tracker| {
                (
                    tracker.avg_insert_latency_ms(),
                    tracker.avg_search_latency_ms(),
                    tracker.avg_lock_read_latency_ms(),
                    tracker.avg_lock_write_latency_ms(),
                )
            })
            .unwrap_or_default();

        total_vectors += count;
        let filter_overfetch = Some(collection_guard.config.search.filter_overfetch);
        let (hnsw_ef_search, ivf_nprobe) = match collection_guard.vector_index().stats().details {
            piramid_database::index::IndexDetails::Flat => (None, None),
            piramid_database::index::IndexDetails::Hnsw { ef_search, .. } => {
                (Some(ef_search), None)
            }
            piramid_database::index::IndexDetails::Ivf { num_probes, .. } => {
                (None, Some(num_probes))
            }
        };

        collection_metrics.push(CollectionMetrics {
            name: collection_name.clone(),
            vector_count: count,
            index_type,
            memory_usage_bytes,
            insert_latency_ms,
            search_latency_ms,
            lock_read_ms,
            lock_write_ms,
            filter_overfetch,
            hnsw_ef_search,
            ivf_nprobe,
        });

        let wal_size = optional_file_size(&SidecarManager::at(&collection_guard.path).wal_path())?;
        let last_checkpoint = collection_guard.checkpoint.last_checkpoint();
        let checkpoint_age_secs = match last_checkpoint {
            Some(timestamp) => piramid_core::clock::unix_secs()?.checked_sub(timestamp),
            None => None,
        };
        wal_stats.push(WalStats {
            collection: collection_name,
            last_checkpoint,
            checkpoint_age_secs,
            wal_size_bytes: wal_size,
        });
    }

    let embed_metrics = state.embeddings.metrics().snapshot();
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
        host: crate::services::convert::host_to_response(state.machine.host()),
        gpus: state
            .machine
            .gpus()
            .into_iter()
            .map(crate::services::convert::gpu_to_response)
            .collect(),
        gpu_budget: state.gpu.as_ref().map(|gpu| {
            let budget = gpu.budget();
            GpuBudgetResponse {
                usable_bytes: budget.usable_bytes(),
                shared: budget.is_shared(),
                pools: budget
                    .usage()
                    .into_iter()
                    .map(|usage| GpuPoolResponse {
                        pool: usage.pool.as_str(),
                        capacity_bytes: usage.capacity_bytes,
                        used_bytes: usage.used_bytes,
                    })
                    .collect(),
            }
        }),
        inference: state.inference.as_ref().map(|manager| {
            let info = manager.info();
            let m = manager.metrics().snapshot();
            InferenceMetricsResponse {
                model: info.name,
                device: info.device,
                requests_admitted: m.requests_admitted,
                requests_finished: m.requests_finished,
                requests_failed: m.requests_failed,
                prompt_tokens: m.prompt_tokens,
                cached_prompt_tokens: m.cached_prompt_tokens,
                generated_tokens: m.generated_tokens,
                avg_time_to_first_token_ms: m.avg_time_to_first_token_ms,
                decode_tokens_per_second: m.decode_tokens_per_second,
                avg_decode_step_ms: m.avg_decode_step_ms,
                prefill_tokens_per_second: m.prefill_tokens_per_second,
                preemptions: m.preemptions,
                queue_depth: m.queue_depth,
                running: m.running,
                last_batch_size: m.last_batch_size,
                kv_blocks_total: m.kv_blocks_total,
                kv_blocks_used: m.kv_blocks_used,
                kv_blocks_cached: m.kv_blocks_cached,
                kv_evictions: m.kv_evictions,
                prefix_hit_rate: m.prefix_hit_rate,
            }
        }),
    })
}

/// Readiness report covering loaded collections, collections on disk only, and disk capacity.
pub fn readyz(state: &SharedState) -> Result<ReadyzResponse> {
    state.ensure_available()?;

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
        let checkpoint_age_secs = match last_checkpoint {
            Some(timestamp) => piramid_core::clock::unix_secs()?.checked_sub(timestamp),
            None => None,
        };
        let wal_size_bytes =
            optional_file_size(&SidecarManager::at(&collection_guard.path).wal_path())?;

        collections.push(CollectionHealth {
            name,
            loaded: true,
            count: Some(count),
            index_type: Some(collection_guard.vector_index().index_type().to_string()),
            last_checkpoint,
            checkpoint_age_secs,
            wal_size_bytes,
            schema_version: Some(collection_guard.manifest.schema_version),
        });
    }

    // Collections load lazily. One present on disk but not yet opened is listed with loaded false.
    for name in state.collection_manager.discover_on_disk()? {
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
        });
    }

    let loaded_collections = state.collection_manager.len();
    let (disk_total_bytes, disk_available_bytes) = crate::disk::stats(&state.data_dir)?;

    Ok(ReadyzResponse {
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

fn optional_file_size(path: &str) -> Result<Option<u64>> {
    match std::fs::metadata(path) {
        Ok(metadata) => Ok(Some(metadata.len())),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}
