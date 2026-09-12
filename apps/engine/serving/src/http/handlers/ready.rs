//! GET /api/readyz.

use axum::{extract::State, response::Json};

use crate::http::ApiResult as Result;
use crate::services::admin;
use crate::services::api::ReadyzResponse;
use crate::state::SharedState;

/// GET /api/readyz: reports readiness and the health of every collection.
pub async fn readyz(State(state): State<SharedState>) -> Result<Json<ReadyzResponse>> {
    Ok(Json(admin::readyz(&state)?))
}
