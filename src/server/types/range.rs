use serde::Deserialize;

fn default_k() -> usize { 10 }

#[derive(Deserialize)]
pub struct RangeSearchRequest {
    pub vector: Vec<f32>,
    pub min_score: f32,
    #[serde(default)]
    pub metric: Option<String>,
    #[serde(default = "default_k")]
    pub k: usize,
    #[serde(default)]
    pub ef: Option<usize>,
    #[serde(default)]
    pub nprobe: Option<usize>,
    #[serde(default)]
    pub overfetch: Option<usize>,
    #[serde(default)]
    pub preset: Option<String>,
}
