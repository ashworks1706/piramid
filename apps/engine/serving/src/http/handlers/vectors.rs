//! Document insert, read, delete, upsert and search endpoints.

use axum::{
    extract::{Extension, Path, Query, State},
    Json,
};

use crate::http::request_id::RequestId;
use crate::http::ApiResult as Result;
use crate::services::api::*;
use crate::services::vector;
use crate::state::SharedState;

/// POST /api/collections/{collection}/vectors: inserts documents, creating the collection
/// if needed.
pub async fn insert_vector(
    State(state): State<SharedState>,
    Path(collection): Path<String>,
    Json(req): Json<InsertRequest>,
) -> Result<Json<InsertResponse>> {
    Ok(Json(vector::insert_vector(&state, collection, req)?))
}

/// GET /api/collections/{collection}/vectors/{id}: returns one document.
pub async fn get_vector(
    State(state): State<SharedState>,
    Path((collection, id)): Path<(String, String)>,
) -> Result<Json<VectorResponse>> {
    Ok(Json(vector::get_vector(&state, collection, id)?))
}

/// GET /api/collections/{collection}/vectors: returns one page of documents.
pub async fn list_vectors(
    State(state): State<SharedState>,
    Path(collection): Path<String>,
    Query(params): Query<ListVectorsQuery>,
) -> Result<Json<Vec<VectorResponse>>> {
    Ok(Json(vector::list_vectors(&state, collection, params)?))
}

/// DELETE /api/collections/{collection}/vectors/{id}: deletes one document.
pub async fn delete_vector(
    State(state): State<SharedState>,
    Path((collection, id)): Path<(String, String)>,
) -> Result<Json<DeleteResponse>> {
    Ok(Json(vector::delete_vector(&state, collection, id)?))
}

/// DELETE /api/collections/{collection}/vectors: deletes documents by id.
pub async fn delete_vectors(
    State(state): State<SharedState>,
    Path(collection): Path<String>,
    Json(req): Json<DeleteVectorsRequest>,
) -> Result<Json<DeleteResponse>> {
    Ok(Json(vector::delete_vectors(&state, collection, req)?))
}

/// POST /api/collections/{collection}/search: searches with one or more query vectors.
pub async fn search_vectors(
    State(state): State<SharedState>,
    Path(collection): Path<String>,
    Extension(request_id): Extension<RequestId>,
    Json(req): Json<SearchRequest>,
) -> Result<Json<SearchResponse>> {
    Ok(Json(vector::search_vectors(
        &state,
        collection,
        request_id.0.as_str(),
        req,
    )?))
}

/// POST /api/collections/{collection}/upsert: inserts or replaces one document, creating the
/// collection if needed.
pub async fn upsert_vector(
    State(state): State<SharedState>,
    Path(collection): Path<String>,
    Json(req): Json<UpsertRequest>,
) -> Result<Json<UpsertResponse>> {
    Ok(Json(vector::upsert_vector(&state, collection, req)?))
}

/// POST /api/collections/{collection}/search/range: searches for hits at or above a minimum score.
pub async fn range_search_vectors(
    State(state): State<SharedState>,
    Path(collection): Path<String>,
    Extension(request_id): Extension<RequestId>,
    Json(req): Json<RangeSearchRequest>,
) -> Result<Json<SearchResponse>> {
    Ok(Json(vector::range_search_vectors(
        &state,
        collection,
        request_id.0.as_str(),
        req,
    )?))
}
