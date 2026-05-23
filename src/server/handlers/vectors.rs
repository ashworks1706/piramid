use axum::{
    extract::{Extension, Path, Query, State},
    Json,
};

use crate::error::Result;
use crate::server::request_id::RequestId;
use crate::server::state::SharedState;
use crate::server::types::range::RangeSearchRequest;
use crate::server::types::*;
use crate::services::vector;

pub async fn insert_vector(
    State(state): State<SharedState>,
    Path(collection): Path<String>,
    Json(req): Json<InsertRequest>,
) -> Result<Json<InsertResultsResponse>> {
    vector::insert_vector(&state, collection, req).map(Json)
}

pub async fn get_vector(
    State(state): State<SharedState>,
    Path((collection, id)): Path<(String, String)>,
) -> Result<Json<VectorResponse>> {
    vector::get_vector(&state, collection, id).map(Json)
}

pub async fn list_vectors(
    State(state): State<SharedState>,
    Path(collection): Path<String>,
    Query(params): Query<ListVectorsQuery>,
) -> Result<Json<Vec<VectorResponse>>> {
    vector::list_vectors(&state, collection, params).map(Json)
}

pub async fn delete_vector(
    State(state): State<SharedState>,
    Path((collection, id)): Path<(String, String)>,
) -> Result<Json<DeleteResultsResponse>> {
    vector::delete_vector(&state, collection, id).map(Json)
}

pub async fn delete_vectors(
    State(state): State<SharedState>,
    Path(collection): Path<String>,
    Json(req): Json<DeleteVectorsRequest>,
) -> Result<Json<DeleteResultsResponse>> {
    vector::delete_vectors(&state, collection, req).map(Json)
}

pub async fn search_vectors(
    State(state): State<SharedState>,
    Path(collection): Path<String>,
    Extension(request_id): Extension<RequestId>,
    Json(req): Json<SearchRequest>,
) -> Result<Json<SearchResultsResponse>> {
    vector::search_vectors(&state, collection, request_id, req).map(Json)
}

pub async fn upsert_vector(
    State(state): State<SharedState>,
    Path(collection): Path<String>,
    Json(req): Json<UpsertRequest>,
) -> Result<Json<UpsertResponse>> {
    vector::upsert_vector(&state, collection, req).map(Json)
}

pub async fn range_search_vectors(
    State(state): State<SharedState>,
    Path(collection): Path<String>,
    Extension(request_id): Extension<RequestId>,
    Json(req): Json<RangeSearchRequest>,
) -> Result<Json<SearchResponse>> {
    vector::range_search_vectors(&state, collection, request_id, req).map(Json)
}
