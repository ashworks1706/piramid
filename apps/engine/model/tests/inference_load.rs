//! Loading an inference manager from configuration, up to the point a checkpoint is needed.
#![allow(clippy::unwrap_used, reason = "assertions in tests")]

use std::path::{Path, PathBuf};
use std::sync::Arc;

use piramid_core::config::{HardwareConfig, InferenceConfig};
use piramid_model::fusion::NoopRetrievalHook;
use piramid_model::inference::InferenceManager;

const CONFIG: &str = r#"{
    "architectures": ["Qwen2ForCausalLM"], "model_type": "qwen2", "vocab_size": 64,
    "hidden_size": 16, "intermediate_size": 32, "num_hidden_layers": 1,
    "num_attention_heads": 2, "num_key_value_heads": 1, "rope_theta": 10000.0,
    "rms_norm_eps": 1e-06, "max_position_embeddings": 128, "tie_word_embeddings": true,
    "hidden_act": "silu", "use_sliding_window": false, "torch_dtype": "float32",
    "eos_token_id": 1, "rope_scaling": null
}"#;

const TOKENIZER_CONFIG: &str = r#"{"chat_template": "{{ messages }}", "eos_token": "</s>"}"#;

fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn the_chat_template_is_read_from_the_tokenizer_path() {
    let weights = scratch("inference_load_weights");
    std::fs::write(weights.join("config.json"), CONFIG).unwrap();
    std::fs::write(weights.join("tokenizer_config.json"), TOKENIZER_CONFIG).unwrap();
    let tokenizer = scratch("inference_load_tokenizer");

    let config = InferenceConfig {
        enabled: true,
        model_path: Some(weights.display().to_string()),
        tokenizer_path: Some(tokenizer.display().to_string()),
        ..InferenceConfig::default()
    };
    let error = InferenceManager::load(
        &config,
        &HardwareConfig::default(),
        None,
        Arc::new(NoopRetrievalHook),
    )
    .unwrap_err();
    let expected = tokenizer
        .join("tokenizer_config.json")
        .display()
        .to_string();
    assert!(error.to_string().contains(&expected), "{error}");
}

#[test]
fn a_tokenizer_path_that_is_not_a_directory_names_the_setting() {
    let weights = scratch("inference_load_file_weights");
    std::fs::write(weights.join("config.json"), CONFIG).unwrap();
    std::fs::write(weights.join("tokenizer_config.json"), TOKENIZER_CONFIG).unwrap();

    let config = InferenceConfig {
        enabled: true,
        model_path: Some(weights.display().to_string()),
        tokenizer_path: Some(weights.join("config.json").display().to_string()),
        ..InferenceConfig::default()
    };
    let error = InferenceManager::load(
        &config,
        &HardwareConfig::default(),
        None,
        Arc::new(NoopRetrievalHook),
    )
    .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("runtime.inference.tokenizer_path"),
        "{error}"
    );
}
