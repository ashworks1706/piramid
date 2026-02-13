use axum::{http::StatusCode, response::Json};
use super::super::{state::SharedState, types::{HealthResponse, MetricsResponse, CollectionMetrics}};
use axum::extract::State;
use crate::error::Result;
use crate::server::types::WalStats;
use crate::server::metrics::record_lock_read;

// GET /api/health - simple liveness check
pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
    })
}

// GET /api/health/embeddings - check if embedding service is available
pub async fn health_embeddings(State(state): State<SharedState>) -> StatusCode {
    match state.embedder {
        Some(_) => StatusCode::OK,
        None => StatusCode::SERVICE_UNAVAILABLE,
    }
}

// GET /api/metrics - basic metrics about collections and vectors
pub async fn metrics(State(state): State<SharedState>) -> Result<Json<MetricsResponse>> {
    let mut collection_metrics = Vec::new();
    let mut wal_stats = Vec::new();
    let mut total_vectors = 0;
    
    for item in state.collections.iter() {
        let collection_name = item.key().clone();
        let storage_ref = item.value();
        
        // Use a read lock with timeout
        let lock_start = std::time::Instant::now();
        let storage = storage_ref.read();
        record_lock_read(state.latency_tracker.get(&collection_name).as_deref(), lock_start);
        let count = storage.count();
        let index_type = storage.vector_index().index_type().to_string();
        let memory_usage_bytes = storage.memory_usage_bytes();
        
        // Get latency stats for this collection
        let (insert_latency_ms, search_latency_ms, lock_read_ms, lock_write_ms) =
            if let Some(tracker) = state.latency_tracker.get(&collection_name) {
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
        let (search_overfetch, hnsw_ef_search, ivf_nprobe) = match &storage.config.index {
            crate::index::IndexConfig::Auto { search, .. } => (Some(search.filter_overfetch), None, None),
            crate::index::IndexConfig::Flat { search, .. } => (Some(search.filter_overfetch), None, None),
            crate::index::IndexConfig::Hnsw { ef_search, search, .. } => (Some(search.filter_overfetch), Some(*ef_search), None),
            crate::index::IndexConfig::Ivf { num_probes, search, .. } => (Some(search.filter_overfetch), None, Some(*num_probes)),
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

        let wal_size = std::fs::metadata(&format!("{}.wal.db", storage.path))
            .map(|m| m.len())
            .ok();
        let checkpoint_age_secs = storage.persistence.last_checkpoint().and_then(|ts| {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .ok()?
                .as_secs();
            now.checked_sub(ts)
        });
        wal_stats.push(WalStats {
            collection: storage.path.clone(),
            last_checkpoint: storage.persistence.last_checkpoint(),
            checkpoint_age_secs,
            wal_size_bytes: wal_size,
        });
    } 
    
    Ok(Json(MetricsResponse {
        total_collections: state.collections.len(),
        total_vectors,
        collections: collection_metrics,
        app_config: state.app_config.clone(),
        wal_stats,
    }))
}
