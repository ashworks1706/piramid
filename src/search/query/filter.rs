// Filters narrow down results by metadata.

use crate::metadata::{Metadata, MetadataValue};

// Chainable filter builder. All conditions must match (AND logic).
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
        self.conditions
            .push(FilterCondition::Eq(field.to_string(), value.into()));
        self
    }

    pub fn ne(mut self, field: &str, value: impl Into<MetadataValue>) -> Self {
        // - `ne`: not equals - field value must not match
        self.conditions
            .push(FilterCondition::Ne(field.to_string(), value.into()));
        self
    }

    pub fn gt(mut self, field: &str, value: impl Into<MetadataValue>) -> Self {
        // - `gt`: greater than - field value must be greater
        self.conditions
            .push(FilterCondition::Gt(field.to_string(), value.into()));
        self
    }

    pub fn gte(mut self, field: &str, value: impl Into<MetadataValue>) -> Self {
        // - `gte`: greater than or equal
        self.conditions
            .push(FilterCondition::Gte(field.to_string(), value.into()));
        self
    }

    pub fn lt(mut self, field: &str, value: impl Into<MetadataValue>) -> Self {
        // - `lt`: less than
        self.conditions
            .push(FilterCondition::Lt(field.to_string(), value.into()));
        self
    }

    pub fn lte(mut self, field: &str, value: impl Into<MetadataValue>) -> Self {
        // - `lte`: less than or equal
        self.conditions
            .push(FilterCondition::Lte(field.to_string(), value.into()));
        self
    }

    pub fn is_in(mut self, field: &str, values: Vec<MetadataValue>) -> Self {
        // - `in`: field value is in the provided list
        self.conditions
            .push(FilterCondition::In(field.to_string(), values));
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

// Individual filter operations
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
            FilterCondition::Eq(field, expected) => metadata.get(field) == Some(expected),
            FilterCondition::Ne(field, expected) => metadata.get(field) != Some(expected),
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
                metadata.get(field).is_some_and(|v| values.contains(v))
            }
        }
    }
}

// Helper to compare numeric metadata values
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
