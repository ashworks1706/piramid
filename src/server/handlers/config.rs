use axum::{extract::State, response::Json};
use std::sync::atomic::Ordering;

use super::super::{
    state::SharedState,
    types::{ConfigReloadResponse, ConfigStatusResponse},
};
use crate::error::{Result, ServerError};

// GET /api/config - return current effective config
pub async fn config_status(State(state): State<SharedState>) -> Result<Json<ConfigStatusResponse>> {
    if state.shutting_down.load(Ordering::Relaxed) {
        return Err(ServerError::ServiceUnavailable("Server is shutting down".to_string()).into());
    }
    let cfg = state.current_config();
    let ts = state.config_last_reload.load(Ordering::Relaxed);
    Ok(Json(ConfigStatusResponse {
        app_config: cfg,
        reloaded_at: Some(ts),
    }))
}

// POST /api/config/reload - hot reload limited subset (currently full AppConfig)
pub async fn reload_config(State(state): State<SharedState>) -> Result<Json<ConfigReloadResponse>> {
    if state.shutting_down.load(Ordering::Relaxed) {
        return Err(ServerError::ServiceUnavailable("Server is shutting down".to_string()).into());
    }
    let cfg = state.reload_config()?;
    let ts = state.config_last_reload.load(Ordering::Relaxed);
    Ok(Json(ConfigReloadResponse {
        success: true,
        reloaded_at: Some(ts),
        app_config: cfg,
    }))
}
