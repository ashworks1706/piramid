#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "assertions in tests"
)]
//! Metadata filter matching.

use {
    piramid_core::metadata::metadata, piramid_core::metadata::Filter,
    piramid_core::metadata::MetadataValue,
};

#[test]
fn filter_matches_eq_and_in() {
    let meta = metadata([
        ("category", "tech".into()),
        ("status", MetadataValue::String("active".into())),
    ]);

    assert!(Filter::new().eq("category", "tech").matches(&meta));
    assert!(!Filter::new().eq("category", "sports").matches(&meta));

    let list = vec!["active".into(), "pending".into()];
    assert!(Filter::new().is_in("status", list).matches(&meta));
}

#[test]
fn filter_numeric_comparisons_work() {
    let meta = metadata([("score", 75i64.into())]);
    assert!(Filter::new().gt("score", 50i64).matches(&meta));
    assert!(Filter::new().lte("score", 75i64).matches(&meta));
    assert!(!Filter::new().gt("score", 80i64).matches(&meta));
}

// matches rejects absent metadata, may_match keeps it as a candidate.
#[test]
fn may_match_admits_absent_metadata_where_matches_rejects_it() {
    let filter = Filter::new().eq("lang", "rust");

    assert!(
        !filter.matches(&metadata([])),
        "an empty document cannot satisfy eq"
    );
    assert!(
        filter.may_match(None),
        "absent metadata must stay a candidate, or an evicted document is lost"
    );
    assert!(filter.may_match(Some(&metadata([("lang", "rust".into())]))));
    assert!(!filter.may_match(Some(&metadata([("lang", "go".into())]))));
}

#[test]
fn may_match_with_present_metadata_is_exactly_matches() {
    let filter = Filter::new().gt("score", 10i64);
    for md in [
        metadata([]),
        metadata([("score", 5i64.into())]),
        metadata([("score", 50i64.into())]),
        metadata([("other", "x".into())]),
    ] {
        assert_eq!(filter.may_match(Some(&md)), filter.matches(&md));
    }
}
