#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "assertions in tests"
)]
//! The generation endpoints over TCP, and the prompt and sampling rules behind them. Without a
//! model they answer 503; with PIRAMID_TEST_MODEL naming a Qwen2.5-0.5B-Instruct directory, the
//! ignored tests generate for real.

use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use piramid_core::config::{Config, SamplingConfig};
use piramid_model::embeddings::EmbeddingsManager;
use piramid_model::inference::tokenizer::ChatMessage;
use piramid_serving::http::serve::{serve, ServeError};
use piramid_serving::services::api::{ChatCompletionRequest, PassageDto, RetrievalDto};
use piramid_serving::services::generation::{
    apply_overrides, chat_retrieval_query, insert_passages, refuse_unsupported, Overrides,
};
use piramid_serving::state::AppState;
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

struct Running {
    addr: SocketAddr,
    shutdown: Option<oneshot::Sender<()>>,
    task: JoinHandle<Result<(), ServeError>>,
}

impl Running {
    fn url(&self, path: &str) -> String {
        format!("http://{}{path}", self.addr)
    }

    async fn stop(mut self) {
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(());
        }
        self.task.await.unwrap().unwrap();
    }
}

fn data_dir(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("generation_{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

async fn start(state: AppState) -> Running {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (tx, rx) = oneshot::channel::<()>();
    let task = tokio::spawn(serve(Arc::new(state), listener, async move {
        let _ = rx.await;
    }));
    Running {
        addr,
        shutdown: Some(tx),
        task,
    }
}

fn config(name: &str) -> Config {
    let mut config = Config::default();
    config.startup.data_dir = data_dir(name).to_string_lossy().into_owned();
    config
}

#[tokio::test]
async fn without_a_model_the_generation_endpoints_say_so() {
    let server =
        start(AppState::new(config("disabled"), EmbeddingsManager::disabled()).unwrap()).await;
    let http = reqwest::Client::new();
    for (path, body) in [
        ("/api/generate", serde_json::json!({"prompt": "hi"})),
        (
            "/v1/chat/completions",
            serde_json::json!({"model": "m", "messages": [{"role": "user", "content": "hi"}]}),
        ),
    ] {
        let response = http
            .post(server.url(path))
            .json(&body)
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), 503, "{path}");
        let text = response.text().await.unwrap();
        assert!(text.contains("runtime.inference.enabled"), "{text}");
    }
    let response = http.get(server.url("/api/model")).send().await.unwrap();
    assert_eq!(response.status(), 503);
    let metrics: serde_json::Value = http
        .get(server.url("/api/metrics"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(metrics.get("inference").is_none());
    server.stop().await;
}

fn passage(text: &str) -> PassageDto {
    PassageDto {
        id: "id".to_string(),
        score: 1.0,
        text: text.to_string(),
    }
}

fn message(role: &str, content: &str) -> ChatMessage {
    ChatMessage {
        role: role.to_string(),
        content: content.to_string(),
    }
}

#[test]
fn passages_join_an_existing_system_message_or_become_one() {
    let passages = [passage("Paris is in France."), passage("Lyon too.")];
    let mut messages = vec![message("user", "Where is Paris?")];
    insert_passages(&mut messages, &passages);
    assert_eq!(messages[0].role, "system");
    assert_eq!(
        messages[0].content,
        "Answer using these passages where they are relevant.\n\n[1] Paris is in France.\n\n[2] Lyon too."
    );

    let mut messages = vec![message("system", "Be brief."), message("user", "q")];
    insert_passages(&mut messages, &passages[..1]);
    assert_eq!(messages.len(), 2);
    assert!(messages[0].content.starts_with("Be brief.\n\nAnswer using"));
}

fn retrieval(query: Option<&str>) -> RetrievalDto {
    RetrievalDto {
        collection: "facts".to_string(),
        k: 1,
        query: query.map(str::to_string),
    }
}

#[test]
fn a_given_retrieval_query_needs_no_user_message() {
    let messages = [message("system", "Be brief.")];
    assert_eq!(
        chat_retrieval_query(&retrieval(Some("vault code")), &messages).ok(),
        Some("vault code")
    );
    assert!(chat_retrieval_query(&retrieval(None), &messages).is_err());
}

#[test]
fn without_a_query_retrieval_searches_with_the_last_user_message() {
    let messages = [
        message("user", "first"),
        message("assistant", "reply"),
        message("user", "second"),
    ];
    assert_eq!(
        chat_retrieval_query(&retrieval(None), &messages).ok(),
        Some("second")
    );
}

fn chat_request(body: serde_json::Value) -> ChatCompletionRequest {
    let mut request = serde_json::json!({
        "model": "m",
        "messages": [{"role": "user", "content": "hi"}]
    });
    if let (Some(request), Some(body)) = (request.as_object_mut(), body.as_object()) {
        request.extend(body.clone());
    }
    serde_json::from_value(request).unwrap()
}

#[test]
fn a_chat_request_naming_a_user_is_refused() {
    assert!(refuse_unsupported(&chat_request(serde_json::json!({}))).is_ok());
    let error = refuse_unsupported(&chat_request(serde_json::json!({"user": "u1"})))
        .unwrap_err()
        .to_string();
    assert!(error.contains("user is not supported"), "{error}");
}

#[test]
fn a_content_part_with_extra_keys_is_refused() {
    let part = |extra: serde_json::Value| {
        let mut part = serde_json::json!({"type": "text", "text": "hi"});
        if let (Some(part), Some(extra)) = (part.as_object_mut(), extra.as_object()) {
            part.extend(extra.clone());
        }
        serde_json::from_value::<ChatCompletionRequest>(serde_json::json!({
            "model": "m",
            "messages": [{"role": "user", "content": [part]}]
        }))
    };
    assert!(part(serde_json::json!({})).is_ok());
    assert!(part(serde_json::json!({"cache_control": {"type": "ephemeral"}})).is_err());
    let error = part(serde_json::json!({"image_url": {"url": "u"}}))
        .unwrap_err()
        .to_string();
    assert!(error.contains("unknown field `image_url`"), "{error}");
}

#[test]
fn overrides_replace_only_what_they_name() {
    let mut sampling = SamplingConfig {
        temperature: 0.7,
        max_new_tokens: 100,
        ..SamplingConfig::default()
    };
    apply_overrides(
        &mut sampling,
        Overrides {
            max_new_tokens: Some(5),
            temperature: None,
            top_p: None,
            top_k: Some(3),
            repetition_penalty: None,
            seed: None,
            stop: None,
        },
    );
    assert_eq!(sampling.max_new_tokens, 5);
    assert_eq!(sampling.temperature, 0.7);
    assert_eq!(sampling.top_k, Some(3));
}

#[cfg(feature = "inference-candle")]
mod with_model {
    use super::*;

    use async_trait::async_trait;
    use piramid_core::config::HardwareConfig;
    use piramid_model::embeddings::{Embedder, EmbeddingResponse, EmbeddingResult};
    use piramid_model::fusion::NoopRetrievalHook;
    use piramid_model::inference::InferenceManager;

    /// Letter-trigram counts hashed into 256 dimensions.
    struct Trigrams;

    #[async_trait]
    impl Embedder for Trigrams {
        async fn embed(&self, text: &str) -> EmbeddingResult<EmbeddingResponse> {
            let mut vector = vec![0.0f32; 256];
            let letters: Vec<u8> = text
                .to_lowercase()
                .bytes()
                .filter(u8::is_ascii_alphanumeric)
                .collect();
            for window in letters.windows(3) {
                let slot = (usize::from(window[0]) * 31 * 31
                    + usize::from(window[1]) * 31
                    + usize::from(window[2]))
                    % 256;
                vector[slot] += 1.0;
            }
            Ok(EmbeddingResponse {
                embedding: vector,
                tokens: None,
                model: "trigrams".to_string(),
            })
        }

        fn provider_name(&self) -> &'static str {
            "trigrams"
        }

        fn model_name(&self) -> &str {
            "trigrams"
        }
    }

    async fn start_with_model(name: &str) -> (Running, String) {
        let mut config = config(name);
        let inference = &mut config.runtime.inference;
        inference.enabled = true;
        inference.model_path =
            Some(std::env::var("PIRAMID_TEST_MODEL").expect("PIRAMID_TEST_MODEL"));
        inference.device =
            Some(std::env::var("PIRAMID_TEST_DEVICE").unwrap_or_else(|_| "cpu".to_string()));
        inference.kv_cache.max_bytes = Some(128 * 1024 * 1024);
        inference.batching.continuous = true;
        let manager = InferenceManager::load(
            inference,
            &HardwareConfig::default(),
            None,
            Arc::new(NoopRetrievalHook),
        )
        .unwrap();
        let model = manager.info().name;
        let state = AppState::new(config, EmbeddingsManager::with_embedder(Arc::new(Trigrams)))
            .unwrap()
            .with_inference(Arc::new(manager))
            .unwrap();
        (start(state).await, model)
    }

    fn sse_data(body: &str) -> Vec<(String, String)> {
        let mut events = Vec::new();
        for block in body.split("\n\n") {
            let mut name = String::from("message");
            let mut data = String::new();
            for line in block.lines() {
                if let Some(value) = line.strip_prefix("event: ") {
                    name = value.to_string();
                } else if let Some(value) = line.strip_prefix("data: ") {
                    data.push_str(value);
                }
            }
            if !data.is_empty() {
                events.push((name, data));
            }
        }
        events
    }

    #[tokio::test(flavor = "multi_thread")]
    #[ignore = "needs PIRAMID_TEST_MODEL"]
    async fn retrieval_before_prefill_grounds_the_answer_and_streams_match() {
        let (server, model) = start_with_model("retrieval").await;
        let http = reqwest::Client::new();

        let stored = http
            .post(server.url("/api/collections/facts/embed"))
            .json(&serde_json::json!({"texts": [
                "The vault on Kestrel Street opens with the code 4817.",
                "Bananas are rich in potassium.",
                "The river Aln flows through Northumberland.",
            ]}))
            .send()
            .await
            .unwrap();
        assert_eq!(stored.status(), 200, "{}", stored.text().await.unwrap());

        let question = "What code opens the vault on Kestrel Street? Reply with the number only.";
        let request = serde_json::json!({
            "messages": [{"role": "user", "content": question}],
            "retrieval": {"collection": "facts", "k": 1},
            "max_new_tokens": 12,
        });
        let answer: serde_json::Value = http
            .post(server.url("/api/generate"))
            .json(&request)
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert!(
            answer["text"].as_str().unwrap().contains("4817"),
            "{answer}"
        );
        assert!(answer["retrieval"]["passages"][0]["text"]
            .as_str()
            .unwrap()
            .contains("Kestrel"));
        assert!(answer["usage"]["time_to_first_token_ms"].as_f64().is_some());

        let mut streamed_request = request.clone();
        streamed_request["stream"] = serde_json::json!(true);
        let body = http
            .post(server.url("/api/generate"))
            .json(&streamed_request)
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap();
        let events = sse_data(&body);
        assert_eq!(events.first().unwrap().0, "retrieval");
        assert_eq!(events.last().unwrap().0, "done");
        let text: String = events
            .iter()
            .filter(|(name, _)| name == "token")
            .map(|(_, data)| {
                serde_json::from_str::<serde_json::Value>(data).unwrap()["text"]
                    .as_str()
                    .unwrap()
                    .to_string()
            })
            .collect();
        assert_eq!(text, answer["text"].as_str().unwrap());

        let chat = serde_json::json!({
            "model": model,
            "messages": [{"role": "user", "content": "Say hello in one word."}],
            "max_tokens": 8,
        });
        let completion: serde_json::Value = http
            .post(server.url("/v1/chat/completions"))
            .json(&chat)
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        let content = completion["choices"][0]["message"]["content"]
            .as_str()
            .unwrap()
            .to_string();
        assert!(!content.is_empty(), "{completion}");
        assert_eq!(completion["object"], "chat.completion");

        let mut streamed = chat.clone();
        streamed["stream"] = serde_json::json!(true);
        streamed["stream_options"] = serde_json::json!({"include_usage": true});
        let body = http
            .post(server.url("/v1/chat/completions"))
            .json(&streamed)
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap();
        let chunks = sse_data(&body);
        assert_eq!(chunks.last().unwrap().1, "[DONE]");
        let mut joined = String::new();
        let mut saw_usage = false;
        for (_, data) in &chunks[..chunks.len() - 1] {
            let chunk: serde_json::Value = serde_json::from_str(data).unwrap();
            if let Some(piece) = chunk["choices"][0]["delta"]["content"].as_str() {
                joined.push_str(piece);
            }
            saw_usage |= chunk["usage"]["completion_tokens"].as_u64().is_some();
        }
        assert_eq!(joined, content);
        assert!(saw_usage);

        let wrong_model = http
            .post(server.url("/v1/chat/completions"))
            .json(&serde_json::json!({"model": "gpt-4", "messages": [{"role": "user", "content": "hi"}]}))
            .send()
            .await
            .unwrap();
        assert_eq!(wrong_model.status(), 404);

        let metrics: serde_json::Value = http
            .get(server.url("/api/metrics"))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert!(metrics["inference"]["generated_tokens"].as_u64().unwrap() > 0);
        let scrape = http
            .get(server.url("/metrics"))
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap();
        assert!(scrape.contains("piramid_inference_generated_tokens_total"));
        assert!(scrape.contains("piramid_kv_blocks_used"));
        server.stop().await;
    }
}
