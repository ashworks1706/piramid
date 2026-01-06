//! Pure vector search returns the most similar vectors regardless of anything else.
//! But usually you want: "similar vectors WHERE category='tech' AND year > 2020"
//! Filters narrow down results by metadata.

use crate::metadata::{Metadata, MetadataValue};

/// Chainable filter builder. All conditions must match (AND logic).
#[derive(Debug, Clone)]
pub struct Filter {
    conditions: Vec<FilterCondition>,
}


impl Filter {
    pub fn new() -> Self {
        Self { conditions: vec![] }
    }
    
    pub fn eq(mut self, field: &str, value: impl Into<MetadataValue>) -> Self {
        // - `eq`: equals - field value must exactly match
        self.conditions.push(FilterCondition::Eq(field.to_string(), value.into()));
        self
    }

    pub fn ne(mut self, field: &str, value: impl Into<MetadataValue>) -> Self {
        // - `ne`: not equals - field value must not match
        self.conditions.push(FilterCondition::Ne(field.to_string(), value.into()));
        self
    }

    pub fn gt(mut self, field: &str, value: impl Into<MetadataValue>) -> Self {
        // - `gt`: greater than - field value must be greater
        self.conditions.push(FilterCondition::Gt(field.to_string(), value.into()));
        self
    }

    pub fn gte(mut self, field: &str, value: impl Into<MetadataValue>) -> Self {
        // - `gte`: greater than or equal
        self.conditions.push(FilterCondition::Gte(field.to_string(), value.into()));
        self
    }

    pub fn lt(mut self, field: &str, value: impl Into<MetadataValue>) -> Self {
        // - `lt`: less than
        self.conditions.push(FilterCondition::Lt(field.to_string(), value.into()));
        self
    }

    pub fn lte(mut self, field: &str, value: impl Into<MetadataValue>) -> Self {
        // - `lte`: less than or equal
        self.conditions.push(FilterCondition::Lte(field.to_string(), value.into()));
        self
    }

    pub fn is_in(mut self, field: &str, values: Vec<MetadataValue>) -> Self {
        // - `in`: field value is in the provided list
        self.conditions.push(FilterCondition::In(field.to_string(), values));
        self
    }

    pub fn matches(&self, metadata: &Metadata) -> bool {
        self.conditions.iter().all(|cond| cond.matches(metadata))
    }

    pub fn is_empty(&self) -> bool {
        self.conditions.is_empty()
    }
}

impl Default for Filter {
    fn default() -> Self {
        Self::new()
    }
}

/// Individual filter operations
#[derive(Debug, Clone)]
pub enum FilterCondition {
    Eq(String, MetadataValue),
    Ne(String, MetadataValue),
    Gt(String, MetadataValue),
    Gte(String, MetadataValue),
    Lt(String, MetadataValue),
    Lte(String, MetadataValue),
    In(String, Vec<MetadataValue>),
}

impl FilterCondition {
    pub fn matches(&self, metadata: &Metadata) -> bool {
        match self {
            FilterCondition::Eq(field, expected) => {
                metadata.get(field).map_or(false, |v| v == expected)
            }
            FilterCondition::Ne(field, expected) => {
                metadata.get(field).map_or(true, |v| v != expected)
            }
            FilterCondition::Gt(field, expected) => {
                compare_values(metadata.get(field), expected, |a, b| a > b)
            }
            FilterCondition::Gte(field, expected) => {
                compare_values(metadata.get(field), expected, |a, b| a >= b)
            }
            FilterCondition::Lt(field, expected) => {
                compare_values(metadata.get(field), expected, |a, b| a < b)
            }
            FilterCondition::Lte(field, expected) => {
                compare_values(metadata.get(field), expected, |a, b| a <= b)
            }
            FilterCondition::In(field, values) => {
                metadata.get(field).map_or(false, |v| values.contains(v))
            }
        }
    }
}

/// Helper to compare numeric metadata values
fn compare_values<F>(actual: Option<&MetadataValue>, expected: &MetadataValue, cmp: F) -> bool
where
    F: Fn(f64, f64) -> bool,
{
    let Some(actual) = actual else {
        return false;
    };
    
    let actual_num = match actual {
        MetadataValue::Integer(i) => *i as f64,
        MetadataValue::Float(f) => *f,
        _ => return false,
    };
    
    let expected_num = match expected {
        MetadataValue::Integer(i) => *i as f64,
        MetadataValue::Float(f) => *f,
        _ => return false,
    };
    
    cmp(actual_num, expected_num)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metadata::metadata;

    #[test]
    fn test_eq_filter() {
        let meta = metadata([("category", "tech".into())]);
        
        let filter = Filter::new().eq("category", "tech");
        assert!(filter.matches(&meta));
        
        let filter = Filter::new().eq("category", "sports");
        assert!(!filter.matches(&meta));
    }

    #[test]
    fn test_numeric_comparisons() {
        let meta = metadata([("score", 75i64.into())]);
        
        assert!(Filter::new().gt("score", 50i64).matches(&meta));
        assert!(!Filter::new().gt("score", 80i64).matches(&meta));
        
        assert!(Filter::new().gte("score", 75i64).matches(&meta));
        assert!(Filter::new().lte("score", 75i64).matches(&meta));
        
        assert!(Filter::new().lt("score", 100i64).matches(&meta));
    }

    #[test]
    fn test_in_filter() {
        let meta = metadata([("status", "active".into())]);
        
        let filter = Filter::new().is_in("status", vec!["active".into(), "pending".into()]);
        assert!(filter.matches(&meta));
        
        let filter = Filter::new().is_in("status", vec!["closed".into()]);
        assert!(!filter.matches(&meta));
    }

    #[test]
    fn test_multiple_conditions() {
        let meta = metadata([
            ("category", "tech".into()),
            ("score", 85i64.into()),
        ]);
        
        let filter = Filter::new()
            .eq("category", "tech")
            .gt("score", 80i64);
        assert!(filter.matches(&meta));
        
        let filter = Filter::new()
            .eq("category", "tech")
            .gt("score", 90i64);
        assert!(!filter.matches(&meta));
    }
}
