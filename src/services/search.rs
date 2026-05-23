use crate::config::SearchConfig;
use crate::metrics::Metric;
use crate::server::{helpers::metadata_to_json, types::HitResponse};
use crate::search::Hit;

pub fn parse_metric(metric: Option<String>) -> Metric {
    match metric.as_deref() {
        Some("euclidean") => Metric::Euclidean,
        Some("dot") | Some("dot_product") => Metric::DotProduct,
        _ => Metric::Cosine,
    }
}

pub fn apply_search_overrides(
    base: SearchConfig,
    req_ef: Option<usize>,
    req_nprobe: Option<usize>,
    req_overfetch: Option<usize>,
    preset: Option<String>,
) -> SearchConfig {
    let mut cfg = base;
    if let Some(preset) = preset {
        match preset.to_lowercase().as_str() {
            "fast" => {
                cfg.ef = Some(50);
                cfg.nprobe = Some(1);
            }
            "high" => {
                cfg.ef = Some(400);
                cfg.nprobe = Some(20);
            }
            _ => {}
        }
    }
    if let Some(ef) = req_ef {
        cfg.ef = Some(ef);
    }
    if let Some(nprobe) = req_nprobe {
        cfg.nprobe = Some(nprobe);
    }
    if let Some(overfetch) = req_overfetch {
        cfg.filter_overfetch = overfetch.max(1);
    }
    cfg
}

pub fn hit_to_response(hit: Hit) -> HitResponse {
    HitResponse {
        id: hit.id.to_string(),
        score: hit.score,
        text: hit.text,
        metadata: metadata_to_json(&hit.metadata),
    }
}
