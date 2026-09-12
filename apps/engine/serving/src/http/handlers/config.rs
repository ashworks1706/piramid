//! Configuration status and reload endpoints.

use axum::{extract::State, response::Json};

use crate::http::ApiResult as Result;
use crate::services::admin;
use crate::services::api::{ConfigReloadResponse, ConfigStatusResponse};
use crate::state::SharedState;

/// GET /api/config: returns the configuration in effect.
pub async fn config_status(State(state): State<SharedState>) -> Result<Json<ConfigStatusResponse>> {
    Ok(Json(admin::config_status(&state)?))
}

/// POST /api/config/reload: reloads configuration from disk and environment.
pub async fn reload_config(State(state): State<SharedState>) -> Result<Json<ConfigReloadResponse>> {
    Ok(Json(admin::reload_config(&state)?))
}
