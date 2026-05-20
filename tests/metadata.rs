use piramid::{metadata, MetadataValue};
use std::f64::consts::PI;

#[test]
fn metadata_helper_builds_map() {
    let meta = metadata([
        ("name", "test".into()),
        ("count", 42i64.into()),
        ("score", PI.into()),
        ("active", true.into()),
    ]);

    assert_eq!(meta.get("name").unwrap().as_string(), Some("test"));
    assert_eq!(meta.get("count").unwrap().as_integer(), Some(42));
    assert!((meta.get("score").unwrap().as_float().unwrap() - PI).abs() < 1e-6);
    assert_eq!(meta.get("active").unwrap().as_boolean(), Some(true));
}

#[test]
fn metadata_value_from_impls() {
    let _: MetadataValue = "hello".into();
    let _: MetadataValue = String::from("world").into();
    let _: MetadataValue = 42i64.into();
    let _: MetadataValue = PI.into();
    let _: MetadataValue = true.into();
}
