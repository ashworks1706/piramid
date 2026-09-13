#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "assertions in tests"
)]
//! Configuration parsing, defaults and validation.

use piramid_core::config::{
    AutoIndexConfig, Config, HardwareProfile, IndexConfig, IndexKind, LogLevel, QuantizationLevel,
    QuantizationStage,
};
use piramid_hardware::compute::Metric;

#[test]
fn the_default_config_round_trips_through_yaml() {
    let cfg = Config::default();
    let yaml = yaml_serde::to_string(&cfg).unwrap();
    let parsed: Config = yaml_serde::from_str(&yaml).unwrap();

    assert_eq!(cfg, parsed);
    assert!(yaml.contains("startup:"));
    assert!(yaml.contains("runtime:"));
}

#[test]
fn an_empty_file_is_all_defaults() {
    let cfg: Config = yaml_serde::from_str("{}").unwrap();

    assert_eq!(cfg, Config::default());
    assert_eq!(cfg.startup.bind, "127.0.0.1:6333");
    assert_eq!(cfg.startup.hardware.profile, HardwareProfile::Auto);
    assert_eq!(cfg.startup.logging.level, LogLevel::Info);
    assert_eq!(cfg.runtime.quantization.stage, QuantizationStage::Disabled);
    assert_eq!(cfg.runtime.search.filter_overfetch, 10);
    cfg.validate().unwrap();
}

#[test]
fn a_partial_file_defaults_the_rest() {
    let yaml = r"
startup:
  bind: 127.0.0.1:7000
runtime:
  search:
    filter_overfetch: 3
";
    let cfg: Config = yaml_serde::from_str(yaml).unwrap();

    assert_eq!(cfg.startup.bind, "127.0.0.1:7000");
    assert_eq!(cfg.runtime.search.filter_overfetch, 3);
    assert!(cfg.runtime.search.parallel);
    assert_eq!(cfg.startup.logging.level, LogLevel::Info);
    cfg.validate().unwrap();
}

#[test]
fn a_misspelled_key_is_an_error_rather_than_a_silent_default() {
    let yaml = r"
runtime:
  search:
    filter_overfech: 3
";
    let err = yaml_serde::from_str::<Config>(yaml)
        .unwrap_err()
        .to_string();
    assert!(err.contains("filter_overfech"), "{err}");
}

#[test]
fn a_setting_in_the_wrong_block_is_an_error() {
    let yaml = r"
runtime:
  bind: 127.0.0.1:7000
";
    assert!(yaml_serde::from_str::<Config>(yaml).is_err());
}

#[test]
fn auto_index_thresholds_are_configurable() {
    let cfg = IndexConfig::Auto {
        metric: Metric::Cosine,
        auto: AutoIndexConfig {
            flat_max_vectors: 5,
            ivf_max_vectors: 10,
            ivf_num_clusters: Some(3),
            ivf_num_probes: Some(2),
            ivf_max_iterations: 4,
            hnsw_m: 8,
            hnsw_ef_construction: 64,
            hnsw_ef_search: 32,
        },
    };

    assert_eq!(cfg.select_type(4), IndexKind::Flat);
    assert_eq!(cfg.select_type(7), IndexKind::Ivf);
    assert_eq!(cfg.select_type(12), IndexKind::Hnsw);
}

#[test]
fn unimplemented_settings_are_rejected_rather_than_ignored() {
    let mut cfg = Config::default();

    for level in [
        QuantizationLevel::Int8,
        QuantizationLevel::Pq { subquantizers: 4 },
        QuantizationLevel::Int4,
        QuantizationLevel::Float16,
    ] {
        cfg.runtime.quantization.level = level;
        let err = cfg.validate().unwrap_err();
        assert!(err.contains("runtime.quantization"), "{err}");
    }

    let mut cfg = Config::default();
    cfg.runtime.quantization.stage = QuantizationStage::Index;
    assert!(cfg.validate().is_err());

    let mut cfg = Config::default();
    cfg.runtime.memory.max_memory_per_collection = Some(1024);
    assert!(cfg.validate().is_err());

    let mut cfg = Config::default();
    cfg.runtime.inference.kv_cache.preemption = piramid_core::config::Preemption::Swap;
    let err = cfg.validate().unwrap_err();
    assert!(err.contains("not implemented"), "{err}");

    let mut cfg = Config::default();
    cfg.runtime.inference.fusion.enabled = true;
    assert!(cfg.validate().unwrap_err().contains("not implemented"));
}

#[test]
fn a_bad_bind_address_is_rejected() {
    let mut cfg = Config::default();
    cfg.startup.bind = "6333".to_string();
    assert!(cfg.validate().unwrap_err().contains("address:port"));
}

// Every setting whose subsystem is unimplemented is refused at startup.
#[test]
fn every_unimplemented_subsystem_refuses_to_start() {
    for (name, mutate) in [
        (
            "fusion",
            Box::new(|c: &mut Config| c.runtime.inference.fusion.enabled = true)
                as Box<dyn Fn(&mut Config)>,
        ),
        (
            "document_kv",
            Box::new(|c: &mut Config| c.runtime.inference.document_kv.enabled = true),
        ),
        (
            "retrieval bandwidth share",
            Box::new(|c: &mut Config| c.startup.hardware.vram.retrieval_bandwidth_share = 0.5),
        ),
        (
            "vector cache bounds",
            Box::new(|c: &mut Config| c.runtime.cache.vectors.entries = Some(100)),
        ),
        (
            "metadata ttl",
            Box::new(|c: &mut Config| c.runtime.cache.metadata.ttl_seconds = Some(60)),
        ),
    ] {
        let mut cfg = Config::default();
        mutate(&mut cfg);
        let error = cfg
            .validate()
            .expect_err(&format!("{name} was accepted but nothing implements it"));
        assert!(
            error.contains("not implemented") || error.contains("not enforced"),
            "{name}: error should say so plainly, got {error}"
        );
    }
}

#[test]
fn a_memory_class_profile_supplies_the_memory_budget() {
    use piramid_core::config::HardwareProfile;

    let mut cfg = Config::default();
    cfg.startup.hardware.profile = HardwareProfile::Memory16Gb;
    assert_eq!(
        cfg.startup.hardware.memory_budget(),
        Some(16 * 1024 * 1024 * 1024)
    );

    // An explicit budget wins over what the class would choose.
    cfg.startup.hardware.memory_budget_bytes = Some(4_000_000_000);
    assert_eq!(cfg.startup.hardware.memory_budget(), Some(4_000_000_000));

    // The cpu-only profile implies no budget.
    cfg.startup.hardware.memory_budget_bytes = None;
    cfg.startup.hardware.profile = HardwareProfile::CpuOnly;
    assert_eq!(cfg.startup.hardware.memory_budget(), None);
}

// A host memory budget is refused, from a memory-class profile or set explicitly.
#[test]
fn a_memory_budget_is_refused_until_it_is_enforced() {
    use piramid_core::config::HardwareProfile;

    for profile in [
        HardwareProfile::Memory8Gb,
        HardwareProfile::Memory16Gb,
        HardwareProfile::Memory32Gb,
    ] {
        let mut cfg = Config::default();
        cfg.startup.hardware.profile = profile;
        assert!(cfg.validate().unwrap_err().contains("not enforced"));
    }
    let mut cfg = Config::default();
    cfg.startup.hardware.memory_budget_bytes = Some(1 << 30);
    assert!(cfg.validate().unwrap_err().contains("not enforced"));
}

// The gpu profile refuses a CPU execution mode.
#[test]
fn the_gpu_profile_refuses_a_cpu_execution_mode() {
    use piramid_core::config::HardwareProfile;
    use piramid_hardware::compute::ExecutionMode;

    for execution in [
        ExecutionMode::Auto,
        ExecutionMode::Scalar,
        ExecutionMode::Simd,
        ExecutionMode::Parallel,
    ] {
        let mut cfg = Config::default();
        cfg.startup.hardware.profile = HardwareProfile::Gpu;
        cfg.runtime.execution = execution;
        assert!(cfg.validate().is_err(), "{execution:?}");
    }
}

#[test]
fn memory_class_profiles_round_trip_through_yaml() {
    for name in ["auto", "cpu-only", "gpu", "8gb", "16gb", "32gb"] {
        let yaml = format!("startup:\n  hardware:\n    profile: {name}\n");
        let cfg: Config = yaml_serde::from_str(&yaml).unwrap();
        assert_eq!(cfg.startup.hardware.profile.as_str(), name);
    }
}

#[test]
fn a_gpu_block_size_that_is_not_a_warp_multiple_is_rejected() {
    let mut cfg = Config::default();
    cfg.startup.hardware.gpu.distance_block_size = 100;
    let error = cfg.validate().unwrap_err();
    assert!(error.contains("distance_block_size"), "{error}");
}

#[test]
fn embedding_options_and_cache_are_validated() {
    let parse = |yaml: &str| yaml_serde::from_str::<Config>(yaml).unwrap().validate();

    let base = "startup:\n  embedding:\n    provider: openai\n    model: m\n";
    parse(base).unwrap();
    parse(&format!("{base}    options:\n      dimensions: 256\n")).unwrap();
    assert!(parse(&format!("{base}    options: [1, 2]\n"))
        .unwrap_err()
        .contains("options"));
    assert!(parse(&format!("{base}    options:\n      model: other\n"))
        .unwrap_err()
        .contains("'model'"));
    assert!(parse(&format!("{base}    cache:\n      entries: 0\n"))
        .unwrap_err()
        .contains("cache.entries"));
    parse(&format!(
        "{base}    cache:\n      enabled: false\n      entries: 0\n"
    ))
    .unwrap();
}

#[test]
fn enabling_inference_needs_a_model_path_and_a_known_device() {
    let mut cfg = Config::default();
    cfg.runtime.inference.enabled = true;
    assert!(cfg.validate().unwrap_err().contains("model_path"));

    cfg.runtime.inference.model_path = Some("/models/qwen".to_string());
    cfg.validate().unwrap();

    for device in ["gpu", "cuda:", "cuda:x"] {
        cfg.runtime.inference.device = Some(device.to_string());
        assert!(cfg
            .validate()
            .unwrap_err()
            .contains("runtime.inference.device"));
    }
    cfg.runtime.inference.device = Some("cuda:0".to_string());
    assert!(cfg.validate().unwrap_err().contains("profile: gpu"));
    cfg.startup.hardware.profile = piramid_core::config::HardwareProfile::Gpu;
    cfg.runtime.inference.device = Some("cuda:1".to_string());
    assert!(cfg.validate().unwrap_err().contains("device_ordinal"));
}

#[test]
fn a_vram_split_must_fit_the_budget() {
    let mut cfg = Config::default();
    cfg.startup.hardware.vram.enabled = true;
    cfg.validate().unwrap();
    cfg.startup.hardware.vram.kv_ratio = 0.5;
    assert!(cfg.validate().unwrap_err().contains("sum"));
}
