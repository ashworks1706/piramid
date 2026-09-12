#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "assertions in tests"
)]
//! Checks that config.example.yaml carries every configuration key with its real default value.

use piramid_core::config::Config;

fn example() -> String {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../config.example.yaml");
    std::fs::read_to_string(path).unwrap()
}

#[test]
fn the_example_file_parses_and_is_exactly_the_defaults() {
    let parsed: Config = yaml_serde::from_str(&example()).unwrap();

    assert_eq!(
        parsed,
        Config::default(),
        "config.example.yaml has drifted from the defaults"
    );
    parsed.validate().unwrap();
}

#[test]
fn the_example_file_documents_every_key() {
    let text = example();
    let yaml = yaml_serde::to_string(&Config::default()).unwrap();

    let missing: Vec<&str> = yaml
        .lines()
        .filter_map(|line| line.trim().split(':').next())
        .filter(|key| !key.is_empty() && !key.starts_with('-'))
        .filter(|key| !text.contains(&format!("{key}:")))
        .collect();

    assert!(missing.is_empty(), "undocumented keys: {missing:?}");
}
