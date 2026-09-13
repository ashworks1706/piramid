//! Error messages and their transport-agnostic classification.

use piramid_core::error::{ConfigError, ErrorKind, PiramidError, SearchError, StorageError};
use piramid_hardware::compute::Metric;

#[test]
fn a_metric_mismatch_is_a_bad_request_naming_both_metrics() {
    let error = PiramidError::from(SearchError::MetricMismatch {
        collection: Metric::Cosine,
        requested: Metric::DotProduct,
    });
    assert_eq!(error.kind(), ErrorKind::BadRequest);
    let message = error.to_string();
    assert!(message.contains("cosine"), "{message}");
    assert!(message.contains("dot"), "{message}");
}

#[test]
fn a_legacy_manifest_names_the_collection_and_asks_for_reingestion() {
    let error = PiramidError::from(StorageError::LegacyManifest {
        collection: "docs".into(),
    });
    let message = error.to_string();
    assert!(message.contains("'docs'"), "{message}");
    assert!(message.contains("Piramid 0.2"), "{message}");
    assert!(message.contains("re-ingested"), "{message}");
}

#[test]
fn a_configuration_error_is_a_bad_request() {
    let error = PiramidError::from(ConfigError::Invalid(
        "runtime.memory changed; it applies when the collection is opened".into(),
    ));
    assert_eq!(error.kind(), ErrorKind::BadRequest);
}

#[test]
fn an_unsupported_manifest_names_the_collection_and_the_version() {
    let error = PiramidError::from(StorageError::UnsupportedManifest {
        collection: "docs".into(),
        found: 3,
    });
    let message = error.to_string();
    assert!(message.contains("'docs'"), "{message}");
    assert!(message.contains("version 3"), "{message}");
}
