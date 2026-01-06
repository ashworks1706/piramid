//! Metadata - extra data you store alongside vectors
//!
//! You often want to filter:
//! "find similar documents, but only from 2024" or "only in category X"
//! That's what metadata is for - key-value pairs attached to each vector.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Values that can be stored in metadata
// Rust enums with data: each variant can hold different types!
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MetadataValue {
    String(String),              // holds a String
    Integer(i64),                // holds an i64
    Float(f64),
    Boolean(bool),
    Array(Vec<MetadataValue>),   // recursive! can hold array of values
    Null,                        // no data
}

impl MetadataValue {
    // These methods try to extract the inner value
    // Return Option<T> because it might not be that type
    pub fn as_string(&self) -> Option<&str> {
        match self {
            MetadataValue::String(s) => Some(s),  // return reference to inner string
            _ => None,                             // _ matches anything else
        }
    }

    pub fn as_integer(&self) -> Option<i64> {
        match self {
            MetadataValue::Integer(i) => Some(*i),  // *i dereferences to copy the i64
            _ => None,
        }
    }

    pub fn as_float(&self) -> Option<f64> {
        match self {
            MetadataValue::Float(f) => Some(*f),
            _ => None,
        }
    }

    pub fn as_boolean(&self) -> Option<bool> {
        match self {
            MetadataValue::Boolean(b) => Some(*b),
            _ => None,
        }
    }
}

//  `From` trait enables `.into()` conversion
// impl From<X> for Y means you can do: let y: Y = x.into();
// This lets us write: metadata([("key", "value".into())])
impl From<String> for MetadataValue {
    fn from(s: String) -> Self {
        MetadataValue::String(s)
    }
}

impl From<&str> for MetadataValue {
    fn from(s: &str) -> Self {
        MetadataValue::String(s.to_string())  // &str -> String (allocates)
    }
}

impl From<i64> for MetadataValue {
    fn from(i: i64) -> Self {
        MetadataValue::Integer(i)
    }
}

impl From<i32> for MetadataValue {
    fn from(i: i32) -> Self {
        MetadataValue::Integer(i as i64)  // `as` for numeric conversions
    }
}

impl From<f64> for MetadataValue {
    fn from(f: f64) -> Self {
        MetadataValue::Float(f)
    }
}

impl From<f32> for MetadataValue {
    fn from(f: f32) -> Self {
        MetadataValue::Float(f as f64)
    }
}

impl From<bool> for MetadataValue {
    fn from(b: bool) -> Self {
        MetadataValue::Boolean(b)
    }
}

// Type alias: `Metadata` is just a shorter name for the HashMap
pub type Metadata = HashMap<String, MetadataValue>;

/// Helper to create metadata inline
//  `const N: usize` is a const generic - array size known at compile time
// This means metadata([...]) works with any size array!
pub fn metadata<const N: usize>(pairs: [(&str, MetadataValue); N]) -> Metadata {
    pairs.into_iter()                          // consume array into iterator
        .map(|(k, v)| (k.to_string(), v))      // convert &str keys to String
        .collect()                              // collect into HashMap
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metadata_creation() {
        let meta = metadata([
            ("name", "test".into()),
            ("count", 42i64.into()),
            ("score", 3.14f64.into()),
            ("active", true.into()),
        ]);

        assert_eq!(meta.get("name").unwrap().as_string(), Some("test"));
        assert_eq!(meta.get("count").unwrap().as_integer(), Some(42));
        assert!((meta.get("score").unwrap().as_float().unwrap() - 3.14).abs() < 1e-6);
        assert_eq!(meta.get("active").unwrap().as_boolean(), Some(true));
    }

    #[test]
    fn test_from_impls() {
        let _: MetadataValue = "hello".into();
        let _: MetadataValue = String::from("world").into();
        let _: MetadataValue = 42i64.into();
        let _: MetadataValue = 42i32.into();
        let _: MetadataValue = 3.14f64.into();
        let _: MetadataValue = 3.14f32.into();
        let _: MetadataValue = true.into();
    }
}
