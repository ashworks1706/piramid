//! Model specs read from checkpoint configuration.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "assertions in tests"
)]

use piramid_model::inference::architecture::{Architecture, ModelSpec, Precision};

const QWEN25: &str = r#"{
    "architectures": ["Qwen2ForCausalLM"], "model_type": "qwen2", "vocab_size": 151936,
    "hidden_size": 896, "intermediate_size": 4864, "num_hidden_layers": 24,
    "num_attention_heads": 14, "num_key_value_heads": 2, "rope_theta": 1000000.0,
    "rms_norm_eps": 1e-06, "max_position_embeddings": 32768, "tie_word_embeddings": true,
    "hidden_act": "silu", "use_sliding_window": false, "torch_dtype": "bfloat16",
    "eos_token_id": 151643, "sliding_window": 32768, "rope_scaling": null
}"#;

#[test]
fn a_qwen25_config_reads_into_a_spec() {
    let spec = ModelSpec::from_json(QWEN25, Some(r#"{"eos_token_id": [151645, 151643]}"#)).unwrap();
    assert_eq!(spec.architecture, Architecture::Qwen2);
    assert_eq!(spec.head_dim, 64);
    assert!(spec.qkv_bias && !spec.qk_norm);
    assert_eq!(spec.stored_precision, Precision::Bf16);
    assert_eq!(spec.eos_token_ids, vec![151643, 151645]);
    assert_eq!(spec.kv_layout(Precision::Bf16).bytes_per_token(), 12_288);
    assert_eq!(spec.parameter_count(), 494_032_768);
}

#[test]
fn unsupported_checkpoints_are_refused_by_name() {
    for (field, value) in [
        ("model_type", r#""llama""#),
        ("use_sliding_window", "true"),
        ("hidden_act", r#""gelu""#),
        ("rope_scaling", r#"{"type": "yarn"}"#),
    ] {
        let mut json: serde_json::Value = serde_json::from_str(QWEN25).unwrap();
        json[field] = serde_json::from_str(value).unwrap();
        let error = ModelSpec::from_json(&json.to_string(), None).unwrap_err();
        let needle = if field == "model_type" {
            "llama"
        } else {
            field
        };
        assert!(error.to_string().contains(needle), "{error}");
    }
}
