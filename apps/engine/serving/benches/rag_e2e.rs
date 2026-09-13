//! End-to-end retrieval-augmented generation: embed, search, fetch, prefill and decode for every
//! question of a dataset under each selected arm, written as JSON with a markdown table.
//!
//! Arms:
//!
//! closed-book: the question alone.
//! before-prefill-http: passages from a separate piramid server through its search/text
//! endpoint, placed before prefill.
//! before-prefill-host: passages from an in-process collection scored by the host strategy.
//! before-prefill-device: passages from an in-process collection scored on the GPU. Refused
//! unless the build has the gpu-cuda feature.
//!
//! Environment:
//!
//! PIRAMID_BENCH_MODEL: checkpoint directory. Required.
//! PIRAMID_BENCH_DATASET: question file, one JSON object per line with id, question, answers,
//! and passages of id, text and gold. Required.
//! PIRAMID_BENCH_EMBEDDING: embedding provider as a JSON object with provider (openai or
//! ollama), model, base_url and optionally api_key. Required. The response cache is off.
//! PIRAMID_BENCH_DEVICE: cpu or cuda:N. Default cpu.
//! PIRAMID_BENCH_QUESTIONS: questions read from the file. Default every question.
//! PIRAMID_BENCH_K: passages retrieved per question. Default 5.
//! PIRAMID_BENCH_ARMS: comma-separated arms, run in the order given. Default closed-book and
//! before-prefill-host.
//! PIRAMID_BENCH_OUT: results path. Default rag_e2e.json in the cargo target directory.
//! PIRAMID_BENCH_SEARCH_URL: an /api/collections/NAME/search/text URL. Required by
//! before-prefill-http. The harness inserts every passage, with its embedding, into that
//! collection before the run, so the collection starts empty and the server embeds queries with
//! the same model and with its embedding cache off.
//! PIRAMID_BENCH_MAX_NEW_TOKENS: tokens generated per question. Default 32.
//! PIRAMID_BENCH_KV_CACHE_BYTES: key/value cache budget. Default 2 GiB.
//! PIRAMID_BENCH_WARMUP: questions run per arm before recording. Default 1.
//!
//! Sampling is greedy. Every passage is embedded once, stored with its dataset id in the
//! passage_id metadata field, and each retrieving arm searches with k. Retrieved passages enter
//! the prompt through the same system message the generate endpoint builds. The in-process search
//! returns documents with their text, so search_ms includes reading them and fetch_ms is the time
//! to take passage ids and texts out of the hits.

#[path = "rag_e2e/dataset.rs"]
mod dataset;
#[path = "rag_e2e/plan.rs"]
mod plan;
#[path = "rag_e2e/report.rs"]
mod report;
#[path = "rag_e2e/scoring.rs"]
mod scoring;

use std::collections::HashMap;
use std::fmt::Display;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use piramid_core::config::{
    CollectionConfig, ExecutionMode, HardwareConfig, InferenceConfig, SamplingConfig,
};
use piramid_core::metadata::{Metadata, MetadataValue};
use piramid_core::Document;
use piramid_database::search::SearchParams;
use piramid_database::{Collection, CollectionOpenOptions};
use piramid_hardware::host::GpuSampler;
use piramid_model::embeddings::{create_embedder, Embedder};
use piramid_model::fusion::NoopRetrievalHook;
use piramid_model::inference::batching::GenerationEvent;
use piramid_model::inference::tokenizer::ChatMessage;
use piramid_model::inference::InferenceManager;
use piramid_serving::services::api::PassageDto;
use piramid_serving::services::generation::insert_passages;
use serde::{Deserialize, Serialize};

use dataset::Question;
use plan::{Arm, Plan};
use report::{ArmSummary, Record};

/// Metadata field holding the dataset id of a stored passage.
const PASSAGE_ID: &str = "passage_id";
/// System message every arm starts from.
const INSTRUCTION: &str = "Answer the question with a short phrase.";
/// Documents per insert.
const INSERT_BATCH: usize = 256;

/// A failed run, reported as its message.
struct Failure(String);

impl std::fmt::Debug for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<String> for Failure {
    fn from(message: String) -> Self {
        Failure(message)
    }
}

/// Prefix an error with what was being done.
trait Context<T> {
    /// The value, or a failure naming what and the error.
    fn context(self, what: &str) -> Result<T, Failure>;
}

impl<T, E: Display> Context<T> for Result<T, E> {
    fn context(self, what: &str) -> Result<T, Failure> {
        self.map_err(|e| Failure(format!("{what}: {e}")))
    }
}

fn main() -> Result<(), Failure> {
    let default_out = Path::new(env!("CARGO_TARGET_TMPDIR")).parent().map_or_else(
        || PathBuf::from("rag_e2e.json"),
        |dir| dir.join("rag_e2e.json"),
    );
    let plan = Plan::from_lookup(|name| std::env::var(name).ok(), default_out)?;
    plan.check_arms(cfg!(feature = "gpu-cuda"))?;
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("start the async runtime")?;
    let results = runtime.block_on(run(&plan))?;
    write_results(&plan.out, &results)?;
    report_to_stdout(&plan.out, &results.table_markdown);
    Ok(())
}

#[allow(
    clippy::print_stdout,
    reason = "the benchmark reports its table on stdout"
)]
fn report_to_stdout(out: &Path, table: &str) {
    println!("{table}\nresults written to {}", out.display());
}

/// Everything a run writes.
#[derive(Serialize)]
struct Results {
    config: ConfigRecord,
    hardware: HardwareRecord,
    model: ModelRecord,
    arms: Vec<ArmSummary>,
    table_markdown: String,
    records: Vec<Record>,
}

/// The settings of a run, without credentials.
#[derive(Serialize)]
struct ConfigRecord {
    model: PathBuf,
    dataset: PathBuf,
    device: String,
    embedding_provider: String,
    embedding_model: String,
    embedding_base_url: Option<String>,
    questions: usize,
    passages: usize,
    k: usize,
    arms: Vec<Arm>,
    search_url: Option<String>,
    max_new_tokens: usize,
    kv_cache_bytes: u64,
    warmup: usize,
    sampling: &'static str,
}

/// The machine a run measured.
#[derive(Serialize)]
struct HardwareRecord {
    cpu: Option<String>,
    logical_cpus: usize,
    gpus: Vec<String>,
}

/// The model a run loaded.
#[derive(Serialize)]
struct ModelRecord {
    name: String,
    architecture: &'static str,
    device: String,
}

/// A passage with the vector stored for it.
struct Embedded {
    id: String,
    text: String,
    vector: Vec<f32>,
}

/// Where a retrieving arm finds passages.
enum Source {
    /// An in-process collection searched with a strategy.
    Local {
        collection: Box<Collection>,
        mode: ExecutionMode,
    },
    /// A piramid server.
    Http {
        client: reqwest::Client,
        url: String,
    },
}

/// Passages found for one question and how long each step took.
struct Retrieved {
    passages: Vec<PassageDto>,
    passage_ids: Vec<String>,
    embed_ms: Option<f64>,
    search_ms: Option<f64>,
    fetch_ms: Option<f64>,
}

async fn run(plan: &Plan) -> Result<Results, Failure> {
    let contents = std::fs::read_to_string(&plan.dataset)
        .context(&format!("read {}", plan.dataset.display()))?;
    let questions = dataset::parse(&contents, plan.questions)?;
    let embedder = create_embedder(&plan.embedding).context("build the embedder")?;
    let passages = embed_passages(embedder.as_ref(), &questions).await?;

    let gpu = open_gpu(plan)?;
    let manager = InferenceManager::load(
        &inference_config(plan),
        &HardwareConfig::default(),
        gpu.as_ref(),
        Arc::new(NoopRetrievalHook),
    )
    .context("load the model")?;
    let info = manager.info();
    let sampling = SamplingConfig {
        temperature: 0.0,
        max_new_tokens: plan.max_new_tokens,
        seed: Some(0),
        ..SamplingConfig::default()
    };

    let scratch = Path::new(env!("CARGO_TARGET_TMPDIR")).join("rag_e2e");
    let mut records = Vec::new();
    for arm in &plan.arms {
        let source = match arm {
            Arm::ClosedBook => None,
            Arm::BeforePrefillHost => {
                Some(local_source(&scratch, *arm, ExecutionMode::Auto.resolve(), &passages).await?)
            }
            Arm::BeforePrefillDevice => {
                Some(local_source(&scratch, *arm, ExecutionMode::Gpu, &passages).await?)
            }
            Arm::BeforePrefillHttp => Some(http_source(plan, &passages).await?),
        };
        for question in questions.iter().take(plan.warmup) {
            answer(
                plan,
                &manager,
                embedder.as_ref(),
                source.as_ref(),
                *arm,
                question,
                &sampling,
            )
            .await?;
        }
        for question in &questions {
            records.push(
                answer(
                    plan,
                    &manager,
                    embedder.as_ref(),
                    source.as_ref(),
                    *arm,
                    question,
                    &sampling,
                )
                .await?,
            );
        }
    }
    manager.shutdown();

    let arms: Vec<ArmSummary> = plan
        .arms
        .iter()
        .map(|arm| report::summarize(arm.as_str(), &records))
        .collect();
    Ok(Results {
        config: ConfigRecord {
            model: plan.model.clone(),
            dataset: plan.dataset.clone(),
            device: plan.device.clone(),
            embedding_provider: plan.embedding.provider.clone(),
            embedding_model: plan.embedding.model.clone(),
            embedding_base_url: plan.embedding.base_url.clone(),
            questions: questions.len(),
            passages: passages.len(),
            k: plan.k,
            arms: plan.arms.clone(),
            search_url: plan.search_url.clone(),
            max_new_tokens: plan.max_new_tokens,
            kv_cache_bytes: plan.kv_cache_bytes,
            warmup: plan.warmup,
            sampling: "greedy",
        },
        hardware: hardware(),
        model: ModelRecord {
            name: info.name,
            architecture: info.architecture,
            device: info.device,
        },
        table_markdown: report::markdown_table(&arms, plan.k),
        arms,
        records,
    })
}

/// Open the device the model or the device arm runs on, and serve the gpu execution mode from it.
#[cfg(feature = "gpu-cuda")]
fn open_gpu(plan: &Plan) -> Result<Option<piramid_hardware::gpu::GpuManager>, Failure> {
    use piramid_hardware::gpu::{BudgetSettings, GpuManager};

    let model_ordinal = plan.device.strip_prefix("cuda:");
    if model_ordinal.is_none() && !plan.arms.contains(&Arm::BeforePrefillDevice) {
        return Ok(None);
    }
    let ordinal = model_ordinal
        .map(str::parse::<usize>)
        .transpose()
        .context("parse PIRAMID_BENCH_DEVICE")?
        .unwrap_or(0);
    let settings = BudgetSettings {
        limit_bytes: None,
        reserve_bytes: 0,
        shares: None,
    };
    let manager = GpuManager::open(ordinal, settings, 1).context("open the GPU")?;
    piramid_hardware::compute::strategies::install_gpu(&manager, 256)
        .context("install the GPU for search")?;
    Ok(Some(manager))
}

/// Without a GPU backend no device is opened; a cuda model device fails at load.
#[cfg(not(feature = "gpu-cuda"))]
fn open_gpu(_plan: &Plan) -> Result<Option<piramid_hardware::gpu::GpuManager>, Failure> {
    Ok(None)
}

fn inference_config(plan: &Plan) -> InferenceConfig {
    let mut config = InferenceConfig {
        enabled: true,
        model_path: Some(plan.model.display().to_string()),
        device: Some(plan.device.clone()),
        ..InferenceConfig::default()
    };
    config.kv_cache.max_bytes = Some(plan.kv_cache_bytes);
    config.kv_cache.prefix_sharing = false;
    config.batching.max_batch_size = 1;
    config
}

async fn embed_passages(
    embedder: &dyn Embedder,
    questions: &[Question],
) -> Result<Vec<Embedded>, Failure> {
    let unique = dataset::unique_passages(questions);
    let mut embedded = Vec::with_capacity(unique.len());
    for passage in unique {
        let response = embedder
            .embed(&passage.text)
            .await
            .context(&format!("embed passage {}", passage.id))?;
        embedded.push(Embedded {
            id: passage.id.clone(),
            text: passage.text.clone(),
            vector: response.embedding,
        });
    }
    Ok(embedded)
}

async fn local_source(
    scratch: &Path,
    arm: Arm,
    mode: ExecutionMode,
    passages: &[Embedded],
) -> Result<Source, Failure> {
    let dir = scratch.join(arm.as_str());
    if dir.exists() {
        std::fs::remove_dir_all(&dir).context(&format!("clear {}", dir.display()))?;
    }
    std::fs::create_dir_all(&dir).context(&format!("create {}", dir.display()))?;
    let path = dir.join("passages.db");
    let config = CollectionConfig {
        execution: mode,
        ..CollectionConfig::default()
    };
    let mut collection = Collection::open_with_options(
        &path.display().to_string(),
        CollectionOpenOptions::from(config),
    )
    .context(&format!("open the {} collection", arm.as_str()))?;
    for batch in passages.chunks(INSERT_BATCH) {
        let documents = batch
            .iter()
            .map(|passage| {
                let metadata: Metadata = HashMap::from([(
                    PASSAGE_ID.to_string(),
                    MetadataValue::String(passage.id.clone()),
                )]);
                Document::with_metadata(passage.vector.clone(), passage.text.clone(), metadata)
            })
            .collect();
        collection.insert_batch(documents).context(&format!(
            "insert passages into the {} collection",
            arm.as_str()
        ))?;
    }
    Ok(Source::Local {
        collection: Box::new(collection),
        mode,
    })
}

/// The body of a vectors insert on the server.
#[derive(Serialize)]
struct InsertBody<'a> {
    vectors: Vec<&'a [f32]>,
    texts: Vec<&'a str>,
    metadata: Vec<HashMap<&'static str, &'a str>>,
}

async fn http_source(plan: &Plan, passages: &[Embedded]) -> Result<Source, Failure> {
    let url = plan
        .search_url
        .clone()
        .ok_or_else(|| Failure("before-prefill-http needs PIRAMID_BENCH_SEARCH_URL".to_string()))?;
    let vectors_url = format!("{}/vectors", plan::collection_url(&url)?);
    let client = reqwest::Client::new();
    for batch in passages.chunks(INSERT_BATCH) {
        let body = InsertBody {
            vectors: batch
                .iter()
                .map(|passage| passage.vector.as_slice())
                .collect(),
            texts: batch.iter().map(|passage| passage.text.as_str()).collect(),
            metadata: batch
                .iter()
                .map(|passage| HashMap::from([(PASSAGE_ID, passage.id.as_str())]))
                .collect(),
        };
        client
            .post(&vectors_url)
            .json(&body)
            .send()
            .await
            .and_then(reqwest::Response::error_for_status)
            .context(&format!("insert passages at {vectors_url}"))?;
    }
    Ok(Source::Http { client, url })
}

/// The part of a search/text response the harness reads.
#[derive(Deserialize)]
struct SearchBody {
    results: Vec<Vec<HitBody>>,
    latency_ms: f64,
}

/// One hit of a search/text response.
#[derive(Deserialize)]
struct HitBody {
    id: String,
    score: f32,
    text: String,
    metadata: HashMap<String, serde_json::Value>,
}

async fn retrieve(
    source: &Source,
    embedder: &dyn Embedder,
    question: &str,
    k: usize,
) -> Result<Retrieved, Failure> {
    match source {
        Source::Local { collection, mode } => {
            let started = Instant::now();
            let embedded = embedder
                .embed(question)
                .await
                .context("embed the question")?;
            let embed_ms = millis(started.elapsed());

            let started = Instant::now();
            let hits = collection
                .search(
                    &embedded.embedding,
                    k,
                    collection.vector_index().metric(),
                    SearchParams {
                        mode: *mode,
                        ..SearchParams::default()
                    },
                )
                .context("search the collection")?;
            let search_ms = millis(started.elapsed());

            let started = Instant::now();
            let mut passages = Vec::with_capacity(hits.len());
            let mut passage_ids = Vec::with_capacity(hits.len());
            for hit in hits {
                let id = hit
                    .document
                    .metadata
                    .get(PASSAGE_ID)
                    .and_then(MetadataValue::as_string)
                    .ok_or_else(|| {
                        Failure(format!("document {} has no {PASSAGE_ID}", hit.document.id))
                    })?
                    .to_string();
                passages.push(PassageDto {
                    id: hit.document.id.to_string(),
                    score: hit.score,
                    text: hit.document.text,
                });
                passage_ids.push(id);
            }
            let fetch_ms = millis(started.elapsed());
            Ok(Retrieved {
                passages,
                passage_ids,
                embed_ms: Some(embed_ms),
                search_ms: Some(search_ms),
                fetch_ms: Some(fetch_ms),
            })
        }
        Source::Http { client, url } => {
            let body: SearchBody = client
                .post(url)
                .json(&serde_json::json!({ "query": question, "k": k }))
                .send()
                .await
                .and_then(reqwest::Response::error_for_status)
                .context(&format!("search at {url}"))?
                .json()
                .await
                .context(&format!("read the search response of {url}"))?;
            let hits = body.results.into_iter().next().unwrap_or_default();
            let mut passages = Vec::with_capacity(hits.len());
            let mut passage_ids = Vec::with_capacity(hits.len());
            for hit in hits {
                let id = hit
                    .metadata
                    .get(PASSAGE_ID)
                    .and_then(serde_json::Value::as_str)
                    .ok_or_else(|| Failure(format!("hit {} has no {PASSAGE_ID}", hit.id)))?
                    .to_string();
                passages.push(PassageDto {
                    id: hit.id,
                    score: hit.score,
                    text: hit.text,
                });
                passage_ids.push(id);
            }
            Ok(Retrieved {
                passages,
                passage_ids,
                embed_ms: None,
                search_ms: Some(body.latency_ms),
                fetch_ms: None,
            })
        }
    }
}

async fn answer(
    plan: &Plan,
    manager: &InferenceManager,
    embedder: &dyn Embedder,
    source: Option<&Source>,
    arm: Arm,
    question: &Question,
    sampling: &SamplingConfig,
) -> Result<Record, Failure> {
    let started = Instant::now();
    let mut messages = vec![
        ChatMessage {
            role: "system".to_string(),
            content: INSTRUCTION.to_string(),
        },
        ChatMessage {
            role: "user".to_string(),
            content: question.question.clone(),
        },
    ];
    let retrieved = match source {
        Some(source) => Some(retrieve(source, embedder, &question.question, plan.k).await?),
        None => None,
    };
    let retrieval_ms = retrieved.as_ref().map(|_| millis(started.elapsed()));
    if let Some(found) = &retrieved {
        insert_passages(&mut messages, &found.passages);
    }

    let prompt = manager
        .render_chat(&messages)
        .context("render the prompt")?;
    let tokens = manager.tokenize(&prompt).context("tokenize the prompt")?;
    let mut generation = manager
        .generate(tokens, sampling.clone())
        .await
        .context("queue the generation")?;
    let mut first_token = None;
    let mut output = String::new();
    let usage = loop {
        match generation.next().await {
            Some(GenerationEvent::Token { text, .. }) => {
                first_token.get_or_insert_with(|| started.elapsed());
                output.push_str(&text);
            }
            Some(GenerationEvent::Finished { usage, .. }) => break usage,
            Some(GenerationEvent::Failed(error)) => {
                return Err(Failure(format!(
                    "generate for question {} under {}: {error}",
                    question.id,
                    arm.as_str()
                )))
            }
            None => {
                return Err(Failure(format!(
                    "the engine ended question {} under {} without a result",
                    question.id,
                    arm.as_str()
                )))
            }
        }
    };

    let gold = question.gold_ids();
    Ok(Record {
        question: question.id.clone(),
        arm: arm.as_str(),
        embed_ms: retrieved.as_ref().and_then(|found| found.embed_ms),
        search_ms: retrieved.as_ref().and_then(|found| found.search_ms),
        fetch_ms: retrieved.as_ref().and_then(|found| found.fetch_ms),
        retrieval_ms,
        prefill_ms: usage.time_to_first_token.map(millis),
        ttft_ms: first_token.map(millis),
        decode_tokens_per_sec: scoring::decode_tokens_per_second(
            usage.completion_tokens,
            usage.time_to_first_token,
            usage.total_time,
        ),
        prompt_tokens: usage.prompt_tokens,
        completion_tokens: usage.completion_tokens,
        recall: retrieved
            .as_ref()
            .and_then(|found| scoring::recall_at_k(&found.passage_ids, &gold)),
        exact_match: scoring::exact_match(&output, &question.answers),
        output,
    })
}

fn millis(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1e3
}

fn hardware() -> HardwareRecord {
    use sysinfo::{CpuRefreshKind, RefreshKind, System};

    let system =
        System::new_with_specifics(RefreshKind::nothing().with_cpu(CpuRefreshKind::nothing()));
    HardwareRecord {
        cpu: system
            .cpus()
            .first()
            .map(|cpu| cpu.brand().trim().to_string())
            .filter(|brand| !brand.is_empty()),
        logical_cpus: system.cpus().len(),
        gpus: GpuSampler::new()
            .sample()
            .into_iter()
            .map(|gpu| gpu.name.unwrap_or_else(|| format!("GPU {}", gpu.index)))
            .collect(),
    }
}

fn write_results(out: &Path, results: &Results) -> Result<(), Failure> {
    if let Some(dir) = out.parent().filter(|dir| !dir.as_os_str().is_empty()) {
        std::fs::create_dir_all(dir).context(&format!("create {}", dir.display()))?;
    }
    let json = serde_json::to_string_pretty(results).context("serialize the results")?;
    std::fs::write(out, json).context(&format!("write {}", out.display()))
}
