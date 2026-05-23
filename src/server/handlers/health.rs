use axum::{extract::State, http::StatusCode, response::Json};

use crate::error::Result;
use crate::runtime::SharedState;
use crate::server::types::{HealthResponse, MetricsResponse};
use crate::services::admin;

pub async fn health() -> Json<HealthResponse> {
    Json(admin::health())
}

pub async fn health_embeddings(State(state): State<SharedState>) -> StatusCode {
    if admin::embeddings_available(&state) {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    }
}

pub async fn metrics(State(state): State<SharedState>) -> Result<Json<MetricsResponse>> {
    admin::metrics(&state).map(Json)
}
