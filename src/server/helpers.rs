use std::collections::HashMap;
use crate::{Metadata, MetadataValue};

// Common error messages
pub const COLLECTION_NOT_FOUND: &str = "Collection not found";
pub const VECTOR_NOT_FOUND: &str = "Vector not found";
pub const EMBEDDING_NOT_CONFIGURED: &str = "Embedding service not configured";

// Convert JSON values to internal Metadata type
pub fn json_to_metadata(json: HashMap<String, serde_json::Value>) -> Metadata {
    let mut metadata = Metadata::new();
    
    for (k, v) in json {
        let value = match v {
            serde_json::Value::String(s) => MetadataValue::String(s),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    MetadataValue::Integer(i)
                } else if let Some(f) = n.as_f64() {
                    MetadataValue::Float(f)
                } else {
                    continue;
                }
            }
            serde_json::Value::Bool(b) => MetadataValue::Boolean(b),
            serde_json::Value::Null => MetadataValue::Null,
            _ => continue,
        };
        metadata.insert(k, value);
    }
    
    metadata
}

// Convert internal Metadata to JSON for responses
pub fn metadata_to_json(metadata: &Metadata) -> HashMap<String, serde_json::Value> {
    metadata
        .iter()
        .map(|(k, v)| {
            let json_val = match v {
                MetadataValue::String(s) => serde_json::Value::String(s.clone()),
                MetadataValue::Integer(i) => serde_json::json!(*i),
                MetadataValue::Float(f) => serde_json::json!(*f),
                MetadataValue::Boolean(b) => serde_json::Value::Bool(*b),
                MetadataValue::Null => serde_json::Value::Null,
                MetadataValue::Array(arr) => {
                    serde_json::Value::Array(arr.iter().map(|item| match item {
                        MetadataValue::String(s) => serde_json::Value::String(s.clone()),
                        MetadataValue::Integer(i) => serde_json::json!(*i),
                        MetadataValue::Float(f) => serde_json::json!(*f),
                        MetadataValue::Boolean(b) => serde_json::Value::Bool(*b),
                        _ => serde_json::Value::Null,
                    }).collect())
                }
            };
            (k.clone(), json_val)
        })
        .collect()
}
