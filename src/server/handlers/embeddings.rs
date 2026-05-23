use axum::{
    extract::{Extension, Path, State},
    Json,
};

use crate::error::Result;
use crate::server::request_id::RequestId;
use crate::server::state::SharedState;
use crate::server::types::*;
use crate::services::embedding;

pub async fn embed_text(
    State(state): State<SharedState>,
    Path(collection): Path<String>,
    Json(req): Json<EmbedRequest>,
) -> Result<Json<EmbedResultsResponse>> {
    embedding::embed_text(&state, collection, req)
        .await
        .map(Json)
}

pub async fn search_by_text(
    State(state): State<SharedState>,
    Path(collection): Path<String>,
    Extension(request_id): Extension<RequestId>,
    Json(req): Json<TextSearchRequest>,
) -> Result<Json<SearchResponse>> {
    embedding::search_by_text(&state, collection, request_id, req)
        .await
        .map(Json)
}
