//! The engine thread run over a tiny model.
#![cfg(feature = "inference-candle")]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "assertions in tests"
)]

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use piramid_core::config::SamplingConfig;
use piramid_core::error::InferenceError;
use piramid_core::stats::InferenceMetrics;
use piramid_model::fusion::NoopRetrievalHook;
use piramid_model::inference::architecture::Architecture;
use piramid_model::inference::backends::candle::qwen::testing::tiny_model;
use piramid_model::inference::batching::scheduler::PlannedEntry;
use piramid_model::inference::batching::worker::{
    run, step_progress, Caller, Command, WorkerContext,
};
use piramid_model::inference::batching::{
    FinishReason, GenerationEvent, Scheduler, SchedulerLimits,
};
use piramid_model::inference::forward::Driver;
use piramid_model::inference::kv_cache::BlockAllocator;
use piramid_model::inference::tokenizer::Tokenizer;
use tokio::sync::{mpsc, oneshot};

struct Letters;

impl Tokenizer for Letters {
    fn encode(&self, text: &str) -> Result<Vec<u32>, InferenceError> {
        Ok(text.bytes().map(|b| u32::from(b) % 90).collect())
    }

    fn encode_with_template(&self, text: &str) -> Result<Vec<u32>, InferenceError> {
        self.encode(text)
    }

    fn decode(&self, tokens: &[u32], _skip_special: bool) -> Result<String, InferenceError> {
        Ok(tokens
            .iter()
            .map(|&t| char::from(b'!' + (t % 90) as u8))
            .collect())
    }

    fn token_id(&self, _token: &str) -> Option<u32> {
        None
    }
}

fn spawn(
    limits: SchedulerLimits,
    blocks: usize,
    prefix_sharing: bool,
) -> (
    mpsc::UnboundedSender<Command>,
    std::thread::JoinHandle<()>,
    Arc<InferenceMetrics>,
) {
    let model = tiny_model(Architecture::Qwen3, 11, blocks * 4);
    let driver = Driver::new(model, Arc::new(NoopRetrievalHook));
    let scheduler = Scheduler::new(limits, BlockAllocator::new(blocks, 4, prefix_sharing));
    let metrics = Arc::new(InferenceMetrics::default());
    let context = WorkerContext {
        tokenizer: Arc::new(Letters),
        eos_token_ids: HashSet::new(),
        metrics: metrics.clone(),
        chunk_tokens: 32,
        max_sequence_length: 200,
    };
    let (commands, receiver) = mpsc::unbounded_channel();
    let thread = std::thread::spawn(move || run(driver, scheduler, context, receiver));
    (commands, thread, metrics)
}

fn limits() -> SchedulerLimits {
    SchedulerLimits {
        max_batch_size: 8,
        max_queue_depth: 16,
        max_batched_tokens: 64,
        continuous: true,
        chunked_prefill: true,
        prefill_chunk_tokens: 5,
        queue_timeout: Some(Duration::from_secs(30)),
        max_sequence_length: 200,
    }
}

fn greedy(max_new_tokens: usize) -> SamplingConfig {
    SamplingConfig {
        max_new_tokens,
        ..SamplingConfig::default()
    }
}

async fn generate(
    commands: &mpsc::UnboundedSender<Command>,
    prompt: Vec<u32>,
    sampling: SamplingConfig,
) -> Result<(Vec<u32>, FinishReason), InferenceError> {
    let (events, mut receiver) = mpsc::unbounded_channel();
    let (reply, admitted) = oneshot::channel();
    commands
        .send(Command::Submit {
            prompt,
            sampling,
            events,
            reply,
        })
        .unwrap();
    admitted.await.unwrap()?;
    let mut tokens = Vec::new();
    while let Some(event) = receiver.recv().await {
        match event {
            GenerationEvent::Token { token, .. } => tokens.push(token),
            GenerationEvent::Finished { reason, usage } => {
                assert_eq!(usage.completion_tokens, tokens.len());
                return Ok((tokens, reason));
            }
            GenerationEvent::Failed(error) => return Err(error),
        }
    }
    Err(InferenceError::Stopped("stream ended".to_string()))
}

#[tokio::test(flavor = "multi_thread")]
async fn concurrent_greedy_generations_match_each_one_alone() {
    let prompts: Vec<Vec<u32>> = vec![(1..12).collect(), vec![40, 41, 42], (20..37).collect()];

    let (commands, thread, _) = spawn(limits(), 64, false);
    let mut alone = Vec::new();
    for prompt in &prompts {
        alone.push(
            generate(&commands, prompt.clone(), greedy(9))
                .await
                .unwrap(),
        );
    }
    let together = generate_all(&commands, &prompts, 9).await;
    assert_eq!(together, alone);
    commands.send(Command::Shutdown).unwrap();
    thread.join().unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn preemption_and_prefix_sharing_leave_greedy_output_unchanged() {
    let prompt: Vec<u32> = (5..30).collect();
    let (commands, thread, _) = spawn(limits(), 64, false);
    let reference = generate(&commands, prompt.clone(), greedy(12))
        .await
        .unwrap();
    commands.send(Command::Shutdown).unwrap();
    thread.join().unwrap();

    let (commands, thread, metrics) = spawn(limits(), 12, false);
    let prompts = vec![prompt.clone(), prompt.clone()];
    for result in generate_all(&commands, &prompts, 12).await {
        assert_eq!(result, reference);
    }
    assert!(
        metrics.snapshot().preemptions > 0,
        "{:?}",
        metrics.snapshot()
    );
    commands.send(Command::Shutdown).unwrap();
    thread.join().unwrap();

    let (commands, thread, metrics) = spawn(limits(), 64, true);
    for _ in 0..2 {
        let result = generate(&commands, prompt.clone(), greedy(12))
            .await
            .unwrap();
        assert_eq!(result, reference);
    }
    assert!(
        metrics.snapshot().cached_prompt_tokens > 0,
        "{:?}",
        metrics.snapshot()
    );
    commands.send(Command::Shutdown).unwrap();
    thread.join().unwrap();
}

async fn generate_all(
    commands: &mpsc::UnboundedSender<Command>,
    prompts: &[Vec<u32>],
    max_new_tokens: usize,
) -> Vec<(Vec<u32>, FinishReason)> {
    let handles: Vec<_> = prompts
        .iter()
        .map(|prompt| {
            let commands = commands.clone();
            let prompt = prompt.clone();
            tokio::spawn(async move {
                generate(&commands, prompt, greedy(max_new_tokens))
                    .await
                    .unwrap()
            })
        })
        .collect();
    let mut out = Vec::new();
    for handle in handles {
        out.push(handle.await.unwrap());
    }
    out
}

#[test]
fn a_planned_sequence_that_is_not_running_fails_the_step() {
    let scheduler: Scheduler<Caller> = Scheduler::new(limits(), BlockAllocator::new(4, 4, false));
    let entries = [PlannedEntry {
        id: 7,
        tokens: 1,
        first_step: false,
        samples: true,
    }];
    let error = step_progress(&scheduler, &entries, 32).unwrap_err();
    assert!(matches!(error, InferenceError::Runtime(_)), "{error}");
    assert!(error
        .to_string()
        .contains("planned sequence 7 is not running"));
}

#[tokio::test(flavor = "multi_thread")]
async fn stop_strings_end_a_generation_and_refusals_come_back_at_admission() {
    let (commands, thread, metrics) = spawn(limits(), 64, false);
    let (tokens, reason) = generate(&commands, vec![1, 2, 3], greedy(6)).await.unwrap();
    assert_eq!(reason, FinishReason::Length);
    let text = Letters.decode(&tokens, true).unwrap();
    let stop = text[2..4].to_string();
    let sampling = SamplingConfig {
        stop: vec![stop],
        ..greedy(6)
    };
    let (stopped, reason) = generate(&commands, vec![1, 2, 3], sampling).await.unwrap();
    assert_eq!(reason, FinishReason::Stop);
    assert!(stopped.len() < tokens.len());

    let error = generate(&commands, vec![1; 190], greedy(20))
        .await
        .unwrap_err();
    assert!(
        matches!(error, InferenceError::InvalidRequest(_)),
        "{error}"
    );
    let error = generate(
        &commands,
        vec![1],
        SamplingConfig {
            top_k: Some(0),
            ..greedy(2)
        },
    )
    .await
    .unwrap_err();
    assert!(error.to_string().contains("top_k"));
    assert_eq!(metrics.snapshot().requests_finished, 2);
    commands.send(Command::Shutdown).unwrap();
    thread.join().unwrap();
}
