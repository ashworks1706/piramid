//! Embed and text-search endpoints.

use axum::{
    extract::{Extension, Path, State},
    Json,
};

use crate::http::request_id::RequestId;
use crate::http::ApiResult as Result;
use crate::services::api::*;
use crate::services::embedding;
use crate::state::SharedState;

/// POST /api/collections/{collection}/embed: embeds texts and stores them as documents.
pub async fn embed_text(
    State(state): State<SharedState>,
    Path(collection): Path<String>,
    Json(req): Json<EmbedRequest>,
) -> Result<Json<EmbedResponse>> {
    Ok(Json(embedding::embed_text(&state, collection, req).await?))
}

/// POST /api/collections/{collection}/search/text: embeds a query text and searches with it.
pub async fn search_by_text(
    State(state): State<SharedState>,
    Path(collection): Path<String>,
    Extension(request_id): Extension<RequestId>,
    Json(req): Json<TextSearchRequest>,
) -> Result<Json<SearchResponse>> {
    Ok(Json(
        embedding::search_by_text(&state, collection, request_id.0.as_str(), req).await?,
    ))
}
