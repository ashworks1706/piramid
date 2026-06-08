use crate::config::SearchConfig;
use crate::error::{Result, ServerError};
use crate::metrics::Metric;
use crate::search::Hit;
use crate::server::{helpers::metadata_to_json, types::HitResponse};

pub fn parse_metric(metric: Option<String>) -> Result<Metric> {
    match metric.as_deref() {
        None | Some("cosine") => Ok(Metric::Cosine),
        Some("euclidean") => Ok(Metric::Euclidean),
        Some("dot") | Some("dot_product") => Ok(Metric::DotProduct),
        Some(other) => Err(ServerError::InvalidRequest(format!(
            "Unknown metric '{other}'. Expected cosine, euclidean, dot, or dot_product"
        ))
        .into()),
    }
}

pub fn apply_search_overrides(
    base: SearchConfig,
    req_ef: Option<usize>,
    req_nprobe: Option<usize>,
    req_overfetch: Option<usize>,
    preset: Option<String>,
) -> Result<SearchConfig> {
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
            other => {
                return Err(ServerError::InvalidRequest(format!(
                    "Unknown search preset '{other}'. Expected fast or high"
                ))
                .into())
            }
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
    Ok(cfg)
}

pub fn hit_to_response(hit: Hit) -> HitResponse {
    HitResponse {
        id: hit.id.to_string(),
        score: hit.score,
        text: hit.text,
        metadata: metadata_to_json(&hit.metadata),
    }
}
