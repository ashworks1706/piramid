use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct QueryBudgetConfig {
    #[serde(default)]
    pub latency_budget_ms: Option<u64>,
    #[serde(default)]
    pub recall_target: Option<f32>,
    #[serde(default)]
    pub max_candidates: Option<usize>,
    #[serde(default)]
    pub max_filtered_candidates: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AdaptiveTuningConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_min_ef")]
    pub min_ef: usize,
    #[serde(default = "default_max_ef")]
    pub max_ef: usize,
    #[serde(default = "default_min_nprobe")]
    pub min_nprobe: usize,
    #[serde(default = "default_max_nprobe")]
    pub max_nprobe: usize,
    #[serde(default = "default_min_filter_overfetch")]
    pub min_filter_overfetch: usize,
    #[serde(default = "default_max_filter_overfetch")]
    pub max_filter_overfetch: usize,
}

impl Default for AdaptiveTuningConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            min_ef: default_min_ef(),
            max_ef: default_max_ef(),
            min_nprobe: default_min_nprobe(),
            max_nprobe: default_max_nprobe(),
            min_filter_overfetch: default_min_filter_overfetch(),
            max_filter_overfetch: default_max_filter_overfetch(),
        }
    }
}

fn default_min_ef() -> usize {
    16
}

fn default_max_ef() -> usize {
    800
}

fn default_min_nprobe() -> usize {
    1
}

fn default_max_nprobe() -> usize {
    64
}

fn default_min_filter_overfetch() -> usize {
    1
}

fn default_max_filter_overfetch() -> usize {
    100
}
