use piramid::{metadata, Filter, MetadataValue};

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
