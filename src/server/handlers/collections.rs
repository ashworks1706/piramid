use axum::{
    extract::{Path, State},
    response::Json,
};

use crate::error::Result;
use crate::runtime::SharedState;
use crate::server::types::*;
use crate::services::collection;

pub async fn list_collections(
    State(state): State<SharedState>,
) -> Result<Json<CollectionsResponse>> {
    collection::list_collections(&state).map(Json)
}

pub async fn create_collection(
    State(state): State<SharedState>,
    Json(req): Json<CreateCollectionRequest>,
) -> Result<Json<CollectionInfo>> {
    collection::create_collection(&state, req).map(Json)
}

pub async fn get_collection(
    State(state): State<SharedState>,
    Path(collection): Path<String>,
) -> Result<Json<CollectionInfo>> {
    collection::get_collection(&state, collection).map(Json)
}

pub async fn delete_collection(
    State(state): State<SharedState>,
    Path(collection): Path<String>,
) -> Result<Json<DeleteResponse>> {
    collection::delete_collection(&state, collection).map(Json)
}

pub async fn collection_count(
    State(state): State<SharedState>,
    Path(collection): Path<String>,
) -> Result<Json<CountResponse>> {
    collection::collection_count(&state, collection).map(Json)
}

pub async fn index_stats(
    State(state): State<SharedState>,
    Path(collection): Path<String>,
) -> Result<Json<IndexStatsResponse>> {
    collection::index_stats(&state, collection).map(Json)
}

pub async fn rebuild_index(
    State(state): State<SharedState>,
    Path(collection): Path<String>,
) -> Result<Json<RebuildIndexResponse>> {
    collection::rebuild_index(&state, collection).map(Json)
}

pub async fn find_duplicates(
    State(state): State<SharedState>,
    Path(collection): Path<String>,
    Json(req): Json<DuplicateRequest>,
) -> Result<Json<DuplicateResponse>> {
    collection::find_duplicates(&state, collection, req).map(Json)
}

pub async fn compact_collection(
    State(state): State<SharedState>,
    Path(collection): Path<String>,
) -> Result<Json<RebuildIndexResponse>> {
    collection::compact_collection(&state, collection).map(Json)
}

pub async fn rebuild_index_status(
    State(state): State<SharedState>,
    Path(collection): Path<String>,
) -> Result<Json<RebuildIndexStatusResponse>> {
    collection::rebuild_index_status(&state, collection).map(Json)
}
