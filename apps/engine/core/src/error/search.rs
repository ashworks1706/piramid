//! Search errors.

use piramid_hardware::compute::Metric;
use thiserror::Error;

/// A search could not be served.
#[derive(Error, Debug)]
pub enum SearchError {
    /// A search named a metric other than the one the collection was created with.
    #[error(
        "the collection is measured by {} and cannot be searched by {}",
        .collection.as_str(),
        .requested.as_str()
    )]
    MetricMismatch {
        /// The metric stored in the collection's manifest.
        collection: Metric,
        /// The metric the search asked for.
        requested: Metric,
    },
}
