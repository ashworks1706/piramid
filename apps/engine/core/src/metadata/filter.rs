//! Metadata predicates: a [Filter] narrows results by document metadata.

use crate::metadata::value::{Metadata, MetadataValue};

/// A set of conditions a document's metadata must all satisfy.
#[derive(Debug, Clone)]
pub struct Filter {
    conditions: Vec<FilterCondition>,
}

impl Filter {
    /// A filter with no conditions, which every document satisfies.
    pub fn new() -> Self {
        Self { conditions: vec![] }
    }

    /// Add a condition that field equals value.
    pub fn eq(mut self, field: &str, value: impl Into<MetadataValue>) -> Self {
        self.conditions
            .push(FilterCondition::Eq(field.to_string(), value.into()));
        self
    }

    /// Add a condition that field does not equal value, satisfied when field is absent.
    pub fn ne(mut self, field: &str, value: impl Into<MetadataValue>) -> Self {
        self.conditions
            .push(FilterCondition::Ne(field.to_string(), value.into()));
        self
    }

    /// Add a condition that field is numeric and greater than value.
    pub fn gt(mut self, field: &str, value: impl Into<MetadataValue>) -> Self {
        self.conditions
            .push(FilterCondition::Gt(field.to_string(), value.into()));
        self
    }

    /// Add a condition that field is numeric and greater than or equal to value.
    pub fn gte(mut self, field: &str, value: impl Into<MetadataValue>) -> Self {
        self.conditions
            .push(FilterCondition::Gte(field.to_string(), value.into()));
        self
    }

    /// Add a condition that field is numeric and less than value.
    pub fn lt(mut self, field: &str, value: impl Into<MetadataValue>) -> Self {
        self.conditions
            .push(FilterCondition::Lt(field.to_string(), value.into()));
        self
    }

    /// Add a condition that field is numeric and less than or equal to value.
    pub fn lte(mut self, field: &str, value: impl Into<MetadataValue>) -> Self {
        self.conditions
            .push(FilterCondition::Lte(field.to_string(), value.into()));
        self
    }

    /// Add a condition that field is present and equals one of values.
    pub fn is_in(mut self, field: &str, values: Vec<MetadataValue>) -> Self {
        self.conditions
            .push(FilterCondition::In(field.to_string(), values));
        self
    }

    /// Whether metadata satisfies every condition.
    pub fn matches(&self, metadata: &Metadata) -> bool {
        self.conditions.iter().all(|cond| cond.matches(metadata))
    }

    /// Whether the filter has no conditions.
    pub fn is_empty(&self) -> bool {
        self.conditions.is_empty()
    }

    /// Whether metadata that may be incomplete still leaves the document a candidate.
    pub fn may_match(&self, metadata: Option<&Metadata>) -> bool {
        metadata.is_none_or(|metadata| self.matches(metadata))
    }
}

impl Default for Filter {
    fn default() -> Self {
        Self::new()
    }
}

/// One condition within a [Filter].
#[derive(Debug, Clone)]
pub enum FilterCondition {
    /// The field equals the value.
    Eq(String, MetadataValue),
    /// The field is absent or does not equal the value.
    Ne(String, MetadataValue),
    /// The field is numeric and greater than the numeric value.
    Gt(String, MetadataValue),
    /// The field is numeric and greater than or equal to the numeric value.
    Gte(String, MetadataValue),
    /// The field is numeric and less than the numeric value.
    Lt(String, MetadataValue),
    /// The field is numeric and less than or equal to the numeric value.
    Lte(String, MetadataValue),
    /// The field is present and equals one of the values.
    In(String, Vec<MetadataValue>),
}

impl FilterCondition {
    /// Whether metadata satisfies this condition.
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

/// Compare two metadata values numerically, coercing integers to floats.
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
