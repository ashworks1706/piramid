//! Tests for support bundle rendering.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "assertions in tests"
)]

use std::collections::BTreeMap;
use std::fs;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};

use piramid::config::Config;
use piramid::document::Document;
use piramid::support::{render, Bundle};
use piramid::Collection;

/// A fresh data directory for the named test.
fn data_dir(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("piramid-support-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// A default configuration over a fresh data directory for the named test.
fn config(name: &str) -> Config {
    let mut config = Config::default();
    config.startup.data_dir = data_dir(name).to_string_lossy().into_owned();
    config
}

/// A bundle of config with no config file named.
fn bundle(config: &Config) -> String {
    render(&Bundle {
        config,
        config_file: None,
    })
}

/// A collection in dir holding one checkpointed document and one only in the write-ahead log.
fn collection_with_a_pending_write(dir: &str, name: &str) {
    let mut collection = Collection::open(&format!("{dir}/{name}.db")).unwrap();
    collection
        .insert(Document::new(vec![1.0, 0.0, 0.0], "first".into()))
        .unwrap();
    collection.checkpoint().unwrap();
    collection
        .insert(Document::new(vec![0.0, 1.0, 0.0], "second".into()))
        .unwrap();
}

/// The bytes of a manifest with schema version 1, as Piramid 0.2 wrote it.
fn legacy_manifest(name: &str) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&1u32.to_le_bytes());
    bytes.extend_from_slice(&(name.len() as u64).to_le_bytes());
    bytes.extend_from_slice(name.as_bytes());
    bytes.extend_from_slice(&1_700_000_000u64.to_le_bytes());
    bytes.extend_from_slice(&1_700_000_500u64.to_le_bytes());
    bytes.push(1);
    bytes.extend_from_slice(&3u64.to_le_bytes());
    bytes.extend_from_slice(&1u64.to_le_bytes());
    bytes
}

/// A hash of the contents of every file in dir, by file name.
fn hash_files(dir: &Path) -> BTreeMap<String, u64> {
    fs::read_dir(dir)
        .unwrap()
        .map(|entry| {
            let entry = entry.unwrap();
            let mut hasher = DefaultHasher::new();
            fs::read(entry.path()).unwrap().hash(&mut hasher);
            (
                entry.file_name().to_string_lossy().into_owned(),
                hasher.finish(),
            )
        })
        .collect()
}

#[test]
fn running_the_bundle_leaves_every_file_in_the_data_directory_unchanged() {
    let dir = data_dir("untouched");
    let dir_str = dir.to_string_lossy().into_owned();
    collection_with_a_pending_write(&dir_str, "docs");
    fs::write(dir.join("legacy.db"), b"").unwrap();
    fs::write(dir.join("legacy.db.manifest.db"), legacy_manifest("legacy")).unwrap();
    let before = hash_files(&dir);
    assert!(before.contains_key("docs.db.wal.db"), "{before:?}");

    let output = data_dir("untouched-output").join("bundle.md");
    let status = std::process::Command::new(env!("CARGO_BIN_EXE_piramid"))
        .arg("support-bundle")
        .arg("--output")
        .arg(&output)
        .arg("--data-dir")
        .arg(&dir)
        .env_remove("CONFIG_FILE")
        .status()
        .unwrap();
    assert!(status.success(), "{status}");

    assert_eq!(hash_files(&dir), before);
    let text = fs::read_to_string(&output).unwrap();
    assert!(text.contains("### docs"), "{text}");
    assert!(text.contains("### legacy"), "{text}");
}

#[test]
fn a_schema_1_manifest_is_reported_and_the_bundle_goes_on() {
    let config = config("legacy");
    let dir = &config.startup.data_dir;
    fs::write(format!("{dir}/legacy.db"), b"").unwrap();
    fs::write(
        format!("{dir}/legacy.db.manifest.db"),
        legacy_manifest("legacy"),
    )
    .unwrap();
    collection_with_a_pending_write(dir, "docs");
    let text = bundle(&config);
    let legacy = text
        .split("### legacy")
        .nth(1)
        .expect("the bundle has a section for legacy");
    assert!(
        legacy.contains("schema_version  1, written by Piramid 0.2"),
        "{text}"
    );
    assert!(text.contains("### docs"), "{text}");
    assert!(text.contains("## Resolved configuration"), "{text}");
}

#[test]
fn bundle_names_the_config_file() {
    let config = config("config-file");
    let file = Path::new("/etc/piramid/config.yaml");
    let named = render(&Bundle {
        config: &config,
        config_file: Some(file),
    });
    assert!(
        named.contains("config_file         /etc/piramid/config.yaml"),
        "{named}"
    );
    assert!(!named.contains("running on defaults"), "{named}");
    let unnamed = bundle(&config);
    assert!(unnamed.contains("config_file         none"), "{unnamed}");
}

#[test]
fn bundle_reports_each_collection_from_its_manifest_as_of_the_last_checkpoint() {
    let config = config("collections");
    let dir = &config.startup.data_dir;
    collection_with_a_pending_write(dir, "docs");
    Collection::open(&format!("{dir}/empty.db")).unwrap();
    let text = bundle(&config);
    let docs = text
        .split("### docs")
        .nth(1)
        .expect("the bundle has a section for docs");
    assert!(docs.contains("documents       1"), "{text}");
    assert!(docs.contains("dimension       3"), "{text}");
    assert!(docs.contains("metric          cosine"), "{text}");
    assert!(docs.contains("schema_version  2"), "{text}");
    assert!(text.contains("as of the last checkpoint"), "{text}");
    let empty = text
        .split("### empty")
        .nth(1)
        .expect("the bundle has a section for empty");
    assert!(
        empty.contains("dimension       none, no vector stored"),
        "{text}"
    );
    assert!(text.contains("collections   2"), "{text}");
    assert!(!text.contains("memory_bytes"), "{text}");
    assert!(!text.contains("search_ms"), "{text}");
}

#[test]
fn bundle_reports_the_inference_settings_and_what_the_build_can_load() {
    let mut config = config("inference");
    let inference = &mut config.runtime.inference;
    inference.enabled = true;
    inference.model_path = Some("/nonexistent/qwen2.5".into());
    inference.dtype = piramid::config::Dtype::Fp16;
    inference.max_sequence_length = 2048;
    inference.batching.continuous = true;
    inference.kv_cache.max_bytes = Some(1 << 30);
    let text = bundle(&config);
    assert!(text.contains("## Inference"), "{text}");
    assert!(text.contains("enabled                  true"), "{text}");
    assert!(
        text.contains("model_path               /nonexistent/qwen2.5 (missing on this machine)"),
        "{text}"
    );
    assert!(
        text.contains("model_name               unset, the model_path directory name"),
        "{text}"
    );
    assert!(
        text.contains("device                   cpu (unset, from the hardware profile)"),
        "{text}"
    );
    assert!(text.contains("dtype                    fp16"), "{text}");
    assert!(text.contains("max_sequence_length      2048"), "{text}");
    assert!(
        text.contains("batching                 continuous on, chunked_prefill off"),
        "{text}"
    );
    assert!(
        text.contains("kv_cache.max_bytes       1073741824"),
        "{text}"
    );
    assert!(text.contains("No model was loaded."), "{text}");
    let refused = text.contains("this build has no inference-candle feature");
    assert_eq!(refused, !cfg!(feature = "inference-candle"), "{text}");
}

#[test]
fn bundle_reports_the_hardware_profile_and_gpu_budget_without_opening_a_device() {
    let mut config = config("hardware");
    let hardware = &mut config.startup.hardware;
    hardware.profile = piramid::config::HardwareProfile::Gpu;
    hardware.gpu_memory_budget_bytes = Some(8 << 30);
    hardware.gpu.device_ordinal = 1;
    hardware.vram.enabled = true;
    let text = bundle(&config);
    assert!(text.contains("profile                  gpu"), "{text}");
    assert!(
        text.contains("gpu_memory_budget_bytes  8589934592"),
        "{text}"
    );
    assert!(text.contains("gpu.device_ordinal       1"), "{text}");
    assert!(
        text.contains("vram split               weights 0.6"),
        "{text}"
    );
    assert!(text.contains("No device was opened."), "{text}");
    // The model device follows the gpu profile when it is unset.
    assert!(
        text.contains("device                   cuda:1 (unset, from the hardware profile)"),
        "{text}"
    );
    assert!(!text.contains("gpu-cuda feature, so"), "{text}");
}

#[test]
fn bundle_reports_the_embedding_provider_or_its_absence() {
    let mut config = config("embedding");
    let none = bundle(&config);
    assert!(none.contains("No provider is configured."), "{none}");

    config.startup.embedding = Some(piramid::config::EmbeddingConfig {
        provider: piramid::config::EmbeddingProvider::Piramid,
        model: "/models/bge-small".into(),
        api_key: None,
        base_url: None,
        options: serde_json::json!({"device": "cuda:0", "dtype": "bf16"}),
        cache: piramid::config::EmbeddingCacheConfig::default(),
        timeout: None,
    });
    let text = bundle(&config);
    assert!(text.contains("provider   piramid"), "{text}");
    assert!(text.contains("model      /models/bge-small"), "{text}");
    assert!(text.contains("device     cuda:0"), "{text}");
    assert!(text.contains("dtype      bf16"), "{text}");
    assert!(text.contains("max_tokens 512"), "{text}");
    assert!(text.contains("cache      on, 10000 entries"), "{text}");
    let refused = text.contains("The piramid provider needs the inference-candle feature");
    assert_eq!(refused, !cfg!(feature = "inference-candle"), "{text}");
}

#[test]
fn bundle_reports_file_sizes_and_leaves_checkpoint_age_to_a_running_server() {
    let config = config("files");
    let dir = &config.startup.data_dir;
    collection_with_a_pending_write(dir, "docs");
    let wal = fs::metadata(format!("{dir}/docs.db.wal.db")).unwrap().len();
    let text = bundle(&config);
    let docs = text
        .split("### docs")
        .nth(1)
        .expect("the bundle has a section for docs");
    assert!(docs.contains(&format!("wal_bytes       {wal}")), "{text}");
    assert!(docs.contains("record_bytes    "), "{text}");
    assert!(docs.contains("offsets_bytes   "), "{text}");
    assert!(!text.contains("checkpoint_age_secs"), "{text}");
    assert!(text.contains("GET /api/metrics"), "{text}");
}
