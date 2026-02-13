use serde::Deserialize;

#[derive(Deserialize)]
pub struct RangeSearchRequest {
    pub vector: Vec<f32>,
    pub min_score: f32,
    #[serde(default)]
    pub metric: Option<String>,
}
