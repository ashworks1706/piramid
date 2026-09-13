//! Conversions between the HTTP request/response shapes and domain types.

use crate::services::api::{GpuMetricsResponse, HitResponse, HostMetricsResponse, SearchTuning};
use piramid_core::config::SearchConfig;
use piramid_core::error::{Result, ServerError};
use piramid_core::metadata::{Filter, Metadata, MetadataValue};
use piramid_core::Hit;
use piramid_hardware::compute::{ComputeError, Metric};
use piramid_hardware::host::{GpuReading, HostReading};
use std::collections::HashMap;

/// Resolve a requested metric name against the metric a collection is indexed by.
///
/// An absent metric is the indexed one. An unknown name is a bad request, and a known name other
/// than the indexed metric is refused by the search itself.
pub fn parse_metric(metric: Option<String>, indexed: Metric) -> Result<Metric> {
    let Some(name) = metric else {
        return Ok(indexed);
    };
    name.parse()
        .map_err(|error: ComputeError| ServerError::InvalidRequest(error.to_string()).into())
}

fn require_nonzero(value: usize, name: &str) -> Result<usize> {
    if value == 0 {
        return Err(ServerError::InvalidRequest(format!("{name} must be >= 1")).into());
    }
    Ok(value)
}

/// Layer per-request tuning onto the configured defaults of a collection.
pub fn apply_search_overrides(base: SearchConfig, tuning: &SearchTuning) -> Result<SearchConfig> {
    let mut cfg = base;
    if let Some(ef) = tuning.ef {
        cfg.ef = Some(require_nonzero(ef, "ef")?);
    }
    if let Some(nprobe) = tuning.nprobe {
        cfg.nprobe = Some(require_nonzero(nprobe, "nprobe")?);
    }
    if let Some(overfetch) = tuning.filter_overfetch {
        cfg.filter_overfetch = require_nonzero(overfetch, "filter_overfetch")?;
    }
    Ok(cfg)
}

/// Build a [Filter] from a map of field name to operator and value.
pub fn parse_filter(
    raw: Option<HashMap<String, HashMap<String, serde_json::Value>>>,
) -> Result<Option<Filter>> {
    let Some(raw) = raw else {
        return Ok(None);
    };

    let mut filter = Filter::new();
    for (field, ops) in raw {
        for (op, value) in ops {
            filter = match op.as_str() {
                "in" => {
                    let serde_json::Value::Array(items) = value else {
                        return Err(ServerError::InvalidRequest(format!(
                            "filter '{field}.in' expects an array"
                        ))
                        .into());
                    };
                    let values = items
                        .into_iter()
                        .map(|item| json_to_metadata_value(&field, item))
                        .collect::<Result<Vec<_>>>()?;
                    filter.is_in(&field, values)
                }
                "eq" | "ne" | "gt" | "gte" | "lt" | "lte" => {
                    let value = json_to_metadata_value(&field, value)?;
                    let numeric =
                        matches!(value, MetadataValue::Integer(_) | MetadataValue::Float(_));
                    if !numeric && matches!(op.as_str(), "gt" | "gte" | "lt" | "lte") {
                        return Err(ServerError::InvalidRequest(format!(
                            "filter '{field}.{op}' expects a number"
                        ))
                        .into());
                    }
                    match op.as_str() {
                        "eq" => filter.eq(&field, value),
                        "ne" => filter.ne(&field, value),
                        "gt" => filter.gt(&field, value),
                        "gte" => filter.gte(&field, value),
                        "lt" => filter.lt(&field, value),
                        _ => filter.lte(&field, value),
                    }
                }
                other => {
                    return Err(ServerError::InvalidRequest(format!(
                        "Unknown filter operator '{other}' on '{field}'. \
                         Expected eq, ne, gt, gte, lt, lte, or in"
                    ))
                    .into())
                }
            };
        }
    }
    Ok(Some(filter))
}

/// Convert a search hit to its wire shape.
pub fn hit_to_response(hit: Hit) -> HitResponse {
    HitResponse {
        id: hit.document.id.to_string(),
        score: hit.score,
        text: hit.document.text,
        metadata: metadata_to_json(&hit.document.metadata),
    }
}

/// Convert one JSON value to a [MetadataValue], rejecting shapes metadata cannot hold.
fn json_to_metadata_value(field: &str, value: serde_json::Value) -> Result<MetadataValue> {
    Ok(match value {
        serde_json::Value::String(s) => MetadataValue::String(s),
        serde_json::Value::Bool(b) => MetadataValue::Boolean(b),
        serde_json::Value::Null => MetadataValue::Null,
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                MetadataValue::Integer(i)
            } else if let Some(f) = n.as_f64() {
                MetadataValue::Float(f)
            } else {
                return Err(ServerError::InvalidRequest(format!(
                    "metadata field '{field}' holds a number outside the i64 and f64 ranges"
                ))
                .into());
            }
        }
        serde_json::Value::Array(items) => MetadataValue::Array(
            items
                .into_iter()
                .map(|item| json_to_metadata_value(field, item))
                .collect::<Result<Vec<_>>>()?,
        ),
        serde_json::Value::Object(_) => {
            return Err(ServerError::InvalidRequest(format!(
                "metadata field '{field}' is an object; metadata holds scalars and arrays only"
            ))
            .into())
        }
    })
}

/// The wire shape of a host reading.
pub fn host_to_response(reading: HostReading) -> HostMetricsResponse {
    HostMetricsResponse {
        cpu_percent: reading.cpu_percent,
        memory_used_bytes: reading.memory_used_bytes,
        memory_total_bytes: reading.memory_total_bytes,
        process_cpu_percent: reading.process_cpu_percent,
        process_resident_bytes: reading.process_resident_bytes,
    }
}

/// The wire shape of a GPU reading.
pub fn gpu_to_response(reading: GpuReading) -> GpuMetricsResponse {
    GpuMetricsResponse {
        index: reading.index,
        name: reading.name,
        memory_used_bytes: reading.memory_used_bytes,
        memory_total_bytes: reading.memory_total_bytes,
        utilization_percent: reading.utilization_percent,
        temperature_celsius: reading.temperature_celsius,
    }
}

/// Convert a JSON object to [Metadata], rejecting nested objects and out-of-range numbers.
pub fn json_to_metadata(json: HashMap<String, serde_json::Value>) -> Result<Metadata> {
    json.into_iter()
        .map(|(k, v)| {
            let value = json_to_metadata_value(&k, v)?;
            Ok((k, value))
        })
        .collect()
}

/// Convert [Metadata] to a JSON object.
pub fn metadata_to_json(metadata: &Metadata) -> HashMap<String, serde_json::Value> {
    metadata
        .iter()
        .map(|(k, v)| (k.clone(), metadata_value_to_json(v)))
        .collect()
}

fn metadata_value_to_json(value: &MetadataValue) -> serde_json::Value {
    match value {
        MetadataValue::String(s) => serde_json::Value::String(s.clone()),
        MetadataValue::Integer(i) => serde_json::json!(*i),
        MetadataValue::Float(f) => serde_json::json!(*f),
        MetadataValue::Boolean(b) => serde_json::Value::Bool(*b),
        MetadataValue::Null => serde_json::Value::Null,
        MetadataValue::Array(items) => {
            serde_json::Value::Array(items.iter().map(metadata_value_to_json).collect())
        }
    }
}
