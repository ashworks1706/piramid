use axum::{extract::State, response::Json};

use crate::error::Result;
use crate::server::state::SharedState;
use crate::server::types::ReadyzResponse;
use crate::services::admin;

pub async fn readyz(State(state): State<SharedState>) -> Result<Json<ReadyzResponse>> {
    admin::readyz(&state).map(Json)
}
