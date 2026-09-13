//! Collection and compaction endpoints.

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

/// POST /api/collections/{collection}/compact: rewrites the record store without dead entries.
pub async fn compact_collection(
    State(state): State<SharedState>,
    Path(collection): Path<String>,
) -> Result<Json<CompactResponse>> {
    Ok(Json(collection::compact_collection(&state, collection)?))
}
