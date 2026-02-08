use axum::{http::StatusCode, response::Json};
use super::super::{state::SharedState, types::{HealthResponse, MetricsResponse, CollectionMetrics}, sync::LockHelper};
use axum::extract::State;
use crate::error::Result;

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
    let mut total_vectors = 0;
    
    for item in state.collections.iter() {
        let collection_name = item.key().clone();
        let storage_ref = item.value();
        
        // Use a read lock with timeout
        let storage = storage_ref.read_with_timeout(std::time::Duration::from_secs(5))?;
        let count = storage.count();
        let index_type = storage.vector_index().index_type().to_string();
        let memory_usage_bytes = storage.memory_usage_bytes();
        
        // Get latency stats for this collection
        let (insert_latency_ms, search_latency_ms) = if let Some(tracker) = state.latency_tracker.get(&collection_name) {
            (tracker.avg_insert_latency_ms(), tracker.avg_search_latency_ms())
        } else {
            (None, None)
        };

        total_vectors += count;
        collection_metrics.push(CollectionMetrics {
            name: collection_name,
            vector_count: count,
            index_type,
            memory_usage_bytes,
            insert_latency_ms,
            search_latency_ms,
        });
    } 
    
    Ok(Json(MetricsResponse {
        total_collections: state.collections.len(),
        total_vectors,
        collections: collection_metrics,
    }))
}

