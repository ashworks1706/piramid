//! Generation through the public manager on a real checkpoint. Needs PIRAMID_TEST_MODEL pointing
//! at a Qwen2.5-0.5B-Instruct directory; run with just test-model.
#![cfg(feature = "inference-candle")]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "assertions in tests"
)]

use std::sync::Arc;

use piramid_core::config::{HardwareConfig, InferenceConfig, SamplingConfig};
use piramid_model::fusion::NoopRetrievalHook;
use piramid_model::inference::batching::FinishReason;
use piramid_model::inference::tokenizer::ChatMessage;
use piramid_model::inference::InferenceManager;

fn fixture() -> serde_json::Value {
    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/qwen2.5-0.5b-instruct.json"
    ))
    .unwrap();
    serde_json::from_str(&text).unwrap()
}

fn config(device: &str) -> InferenceConfig {
    let mut config = InferenceConfig {
        enabled: true,
        model_path: Some(std::env::var("PIRAMID_TEST_MODEL").expect("PIRAMID_TEST_MODEL")),
        device: Some(device.to_string()),
        ..InferenceConfig::default()
    };
    config.kv_cache.max_bytes = Some(256 * 1024 * 1024);
    config.batching.continuous = true;
    config
}

async fn check_reference(device: &str) {
    let manager = InferenceManager::load(
        &config(device),
        &HardwareConfig::default(),
        Arc::new(NoopRetrievalHook),
    )
    .unwrap();
    let fixture = fixture();
    let messages = vec![ChatMessage {
        role: "user".to_string(),
        content: "What is the capital of France? Answer in one sentence.".to_string(),
    }];
    let prompt = manager.render_chat(&messages).unwrap();
    assert_eq!(prompt, fixture["prompt"].as_str().unwrap());
    let tokens = manager.tokenize(&prompt).unwrap();
    let expected_ids: Vec<u32> = fixture["prompt_ids"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_u64().unwrap() as u32)
        .collect();
    assert_eq!(tokens, expected_ids);

    let sampling = SamplingConfig {
        max_new_tokens: 16,
        ..SamplingConfig::default()
    };
    let completion = manager
        .generate(tokens.clone(), sampling.clone())
        .await
        .unwrap()
        .collect()
        .await
        .unwrap();
    assert_eq!(completion.reason, FinishReason::Stop);
    assert_eq!(completion.text, "The capital of France is Paris.");
    assert_eq!(completion.usage.prompt_tokens, tokens.len());

    let handles: Vec<_> = (0..4)
        .map(|_| manager.generate(tokens.clone(), sampling.clone()))
        .collect();
    let mut generations = Vec::new();
    for handle in handles {
        generations.push(handle.await.unwrap());
    }
    for generation in generations {
        let again = generation.collect().await.unwrap();
        assert_eq!(again.text, completion.text);
        assert!(again.usage.cached_prompt_tokens > 0);
    }
    let metrics = manager.metrics().snapshot();
    assert_eq!(metrics.requests_finished, 5);
    assert!(metrics.decode_tokens_per_second.is_some());
    manager.shutdown();
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs PIRAMID_TEST_MODEL"]
async fn the_manager_reproduces_the_reference_on_the_cpu() {
    check_reference("cpu").await;
}

#[cfg(feature = "gpu-cuda")]
#[tokio::test(flavor = "multi_thread")]
#[ignore = "needs PIRAMID_TEST_MODEL and a CUDA device"]
async fn the_manager_reproduces_the_reference_on_cuda() {
    check_reference("cuda:0").await;
}
