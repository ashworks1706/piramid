use axum::{extract::State, response::Json};

use crate::error::Result;
use crate::runtime::SharedState;
use crate::server::types::{ConfigReloadResponse, ConfigStatusResponse};
use crate::services::admin;

pub async fn config_status(State(state): State<SharedState>) -> Result<Json<ConfigStatusResponse>> {
    admin::config_status(&state).map(Json)
}

pub async fn reload_config(State(state): State<SharedState>) -> Result<Json<ConfigReloadResponse>> {
    admin::reload_config(&state).map(Json)
}
