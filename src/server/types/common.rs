use serde::Serialize;

#[derive(Serialize)]
pub struct DeleteResponse {
    pub deleted: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<f32>,
}

#[derive(Serialize)]
pub struct CountResponse {
    pub count: usize,
}
