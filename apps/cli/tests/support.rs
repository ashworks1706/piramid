//! Tests for support bundle rendering.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "assertions in tests"
)]

use std::path::Path;
use std::sync::Arc;

use piramid::config::Config;
use piramid::embeddings::EmbeddingsManager;
use piramid::state::AppState;
use piramid::support::{render, Bundle};

fn state(name: &str) -> (Config, Arc<AppState>) {
    let dir = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("piramid-support-{name}-{}", std::process::id()));
    let mut config = Config::default();
    config.startup.data_dir = dir.to_string_lossy().into_owned();
    let state = Arc::new(AppState::new(config.clone(), EmbeddingsManager::disabled()).unwrap());
    (config, state)
}

#[test]
fn bundle_lists_collections_that_failed_to_open() {
    let (config, state) = state("failed");
    let failed = vec![("broken".to_string(), "manifest is corrupt".to_string())];
    let text = render(&Bundle {
        config: &config,
        config_file: None,
        state: &state,
        failed_collections: &failed,
    });
    assert!(
        text.contains("## Collections that failed to open"),
        "{text}"
    );
    assert!(text.contains("broken: manifest is corrupt"), "{text}");
}

#[test]
fn bundle_names_the_config_file() {
    let (config, state) = state("config-file");
    let file = Path::new("/etc/piramid/config.yaml");
    let named = render(&Bundle {
        config: &config,
        config_file: Some(file),
        state: &state,
        failed_collections: &[],
    });
    assert!(
        named.contains("config_file         /etc/piramid/config.yaml"),
        "{named}"
    );
    assert!(!named.contains("running on defaults"), "{named}");
    let unnamed = render(&Bundle {
        config: &config,
        config_file: None,
        state: &state,
        failed_collections: &[],
    });
    assert!(unnamed.contains("config_file         none"), "{unnamed}");
    assert!(!unnamed.contains("failed to open"), "{unnamed}");
}
