//! Collection, index, compaction and duplicate-scan endpoints.

use axum::{
    extract::{Path, State},
    response::Json,
};

use crate::http::ApiResult as Result;
use crate::services::api::*;
use crate::services::collection;
use crate::state::SharedState;

/// GET /api/collections: lists the collections loaded in memory.
pub async fn list_collections(
    State(state): State<SharedState>,
) -> Result<Json<CollectionsResponse>> {
    Ok(Json(collection::list_collections(&state)?))
}

/// POST /api/collections: creates a collection, or opens it if it already exists.
pub async fn create_collection(
    State(state): State<SharedState>,
    Json(req): Json<CreateCollectionRequest>,
) -> Result<Json<CollectionInfo>> {
    Ok(Json(collection::create_collection(&state, req)?))
}

/// GET /api/collections/{collection}: returns the summary of one collection.
pub async fn get_collection(
    State(state): State<SharedState>,
    Path(collection): Path<String>,
) -> Result<Json<CollectionInfo>> {
    Ok(Json(collection::get_collection(&state, collection)?))
}

/// DELETE /api/collections/{collection}: unloads a collection and removes its files.
pub async fn delete_collection(
    State(state): State<SharedState>,
    Path(collection): Path<String>,
) -> Result<Json<DeleteCollectionResponse>> {
    Ok(Json(collection::delete_collection(&state, collection)?))
}

/// GET /api/collections/{collection}/count: returns the number of stored documents.
pub async fn collection_count(
    State(state): State<SharedState>,
    Path(collection): Path<String>,
) -> Result<Json<CountResponse>> {
    Ok(Json(collection::collection_count(&state, collection)?))
}

/// GET /api/collections/{collection}/index/stats: returns statistics of the vector index.
pub async fn index_stats(
    State(state): State<SharedState>,
    Path(collection): Path<String>,
) -> Result<Json<IndexStatsResponse>> {
    Ok(Json(collection::index_stats(&state, collection)?))
}

/// POST /api/collections/{collection}/index/rebuild: starts a background index rebuild.
pub async fn rebuild_index(
    State(state): State<SharedState>,
    Path(collection): Path<String>,
) -> Result<Json<RebuildIndexResponse>> {
    Ok(Json(collection::rebuild_index(&state, collection)?))
}

/// POST /api/collections/{collection}/duplicates: finds near-identical document pairs.
pub async fn find_duplicates(
    State(state): State<SharedState>,
    Path(collection): Path<String>,
    Json(req): Json<DuplicateRequest>,
) -> Result<Json<DuplicateResponse>> {
    Ok(Json(collection::find_duplicates(&state, collection, req)?))
}

/// POST /api/collections/{collection}/compact: rewrites the record store without dead entries.
pub async fn compact_collection(
    State(state): State<SharedState>,
    Path(collection): Path<String>,
) -> Result<Json<RebuildIndexResponse>> {
    Ok(Json(collection::compact_collection(&state, collection)?))
}

/// GET /api/collections/{collection}/index/rebuild/status: reports the most recent index rebuild.
pub async fn rebuild_index_status(
    State(state): State<SharedState>,
    Path(collection): Path<String>,
) -> Result<Json<RebuildIndexStatusResponse>> {
    Ok(Json(collection::rebuild_index_status(&state, collection)?))
}
