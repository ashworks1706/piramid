//! Generation endpoints: /api/generate, /api/model, and OpenAI-compatible /v1 routes.

use axum::extract::State;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use axum::Json;
use futures::stream::{self, Stream, StreamExt};
use piramid_model::inference::batching::GenerationEvent;
use piramid_model::inference::manager::Generation;

use crate::http::ApiResult as Result;
use crate::services::api::{
    ChatCompletionChunk, ChatCompletionRequest, ChatCompletionResponse, DoneEventDto,
    GenerateRequest, GenerateResponse, ModelResponse, OpenAiChoice, OpenAiChunkChoice, OpenAiDelta,
    OpenAiModel, OpenAiModelList, OpenAiResponseMessage, OpenAiUsage, TokenEventDto,
};
use crate::services::generation;
use crate::state::SharedState;

/// GET /api/model: the loaded model.
pub async fn model(State(state): State<SharedState>) -> Result<Json<ModelResponse>> {
    Ok(Json(generation::model(&state)?))
}

/// POST /api/generate: generate from a prompt or conversation, as JSON or server-sent events.
pub async fn generate(
    State(state): State<SharedState>,
    Json(request): Json<GenerateRequest>,
) -> Result<Response> {
    let stream_events = request.stream;
    let started = generation::start(&state, request).await?;
    if stream_events {
        let retrieval = started
            .retrieval
            .map(|found| json_event("retrieval", &found));
        let events = stream::iter(retrieval).chain(native_events(started.generation));
        return Ok(Sse::new(events)
            .keep_alive(KeepAlive::default())
            .into_response());
    }
    let completion = started.generation.collect().await?;
    Ok(Json(GenerateResponse {
        text: completion.text,
        finish_reason: completion.reason.as_str(),
        usage: generation::usage(&completion.usage),
        retrieval: started.retrieval,
    })
    .into_response())
}

/// One server-sent event, or the serialization error that ends the stream.
type SseItem = std::result::Result<Event, axum::Error>;

fn native_events(generation: Generation) -> impl Stream<Item = SseItem> {
    stream::unfold(Some(generation), |state| async move {
        let mut generation = state?;
        let event = generation.next().await?;
        Some(match event {
            GenerationEvent::Token { token, text } => (
                json_event("token", &TokenEventDto { token, text }),
                Some(generation),
            ),
            GenerationEvent::Finished { reason, usage } => (
                json_event(
                    "done",
                    &DoneEventDto {
                        finish_reason: reason.as_str(),
                        usage: generation::usage(&usage),
                    },
                ),
                None,
            ),
            GenerationEvent::Failed(error) => (
                json_event("error", &serde_json::json!({ "error": error.to_string() })),
                None,
            ),
        })
    })
}

fn json_event<T: serde::Serialize>(name: &str, body: &T) -> SseItem {
    Event::default().event(name).json_data(body)
}

/// GET /v1/models: the loaded model in OpenAI form.
pub async fn openai_models(State(state): State<SharedState>) -> Result<Json<OpenAiModelList>> {
    let model = generation::model(&state)?;
    Ok(Json(OpenAiModelList {
        object: "list",
        data: vec![OpenAiModel {
            id: model.name,
            object: "model",
            created: state.inference_loaded_at,
            owned_by: "piramid",
        }],
    }))
}

/// POST /v1/chat/completions: an OpenAI-compatible chat completion.
pub async fn openai_chat_completions(
    State(state): State<SharedState>,
    Json(request): Json<ChatCompletionRequest>,
) -> Result<Response> {
    let stream_chunks = request.stream;
    let include_usage = request
        .stream_options
        .as_ref()
        .is_some_and(|options| options.include_usage);
    let (model, generation) = generation::start_chat(&state, request).await?;
    let id = format!("chatcmpl-{}", generation.id);
    let created = piramid_core::clock::unix_secs()?;
    if stream_chunks {
        let header = ChunkHeader { id, model, created };
        let chunks = openai_chunks(generation, header, include_usage);
        return Ok(Sse::new(chunks)
            .keep_alive(KeepAlive::default())
            .into_response());
    }
    let completion = generation.collect().await?;
    Ok(Json(ChatCompletionResponse {
        id,
        object: "chat.completion",
        created,
        model,
        choices: vec![OpenAiChoice {
            index: 0,
            message: OpenAiResponseMessage {
                role: "assistant",
                content: completion.text,
            },
            finish_reason: completion.reason.as_str(),
        }],
        usage: generation::openai_usage(&completion.usage),
    })
    .into_response())
}

/// Fields shared by every chunk of one streamed completion.
struct ChunkHeader {
    id: String,
    model: String,
    created: u64,
}

impl ChunkHeader {
    fn event(&self, choices: Vec<OpenAiChunkChoice>, usage: Option<OpenAiUsage>) -> SseItem {
        Event::default().json_data(ChatCompletionChunk {
            id: &self.id,
            object: "chat.completion.chunk",
            created: self.created,
            model: &self.model,
            choices,
            usage,
        })
    }
}

enum ChunkState {
    Streaming { generation: Generation, first: bool },
    Usage(OpenAiUsage),
    Done,
    Ended,
}

fn openai_chunks(
    generation: Generation,
    header: ChunkHeader,
    include_usage: bool,
) -> impl Stream<Item = SseItem> {
    let start = ChunkState::Streaming {
        generation,
        first: true,
    };
    stream::unfold((header, start), move |(header, state)| async move {
        let (event, next) = match state {
            ChunkState::Streaming {
                mut generation,
                first,
            } => match generation.next().await? {
                GenerationEvent::Token { text, .. } => (
                    header.event(
                        vec![OpenAiChunkChoice {
                            index: 0,
                            delta: OpenAiDelta {
                                role: first.then_some("assistant"),
                                content: Some(text),
                            },
                            finish_reason: None,
                        }],
                        None,
                    ),
                    ChunkState::Streaming {
                        generation,
                        first: false,
                    },
                ),
                GenerationEvent::Finished { reason, usage } => (
                    header.event(
                        vec![OpenAiChunkChoice {
                            index: 0,
                            delta: OpenAiDelta {
                                role: first.then_some("assistant"),
                                content: None,
                            },
                            finish_reason: Some(reason.as_str()),
                        }],
                        None,
                    ),
                    if include_usage {
                        ChunkState::Usage(generation::openai_usage(&usage))
                    } else {
                        ChunkState::Done
                    },
                ),
                GenerationEvent::Failed(error) => (
                    Event::default().json_data(serde_json::json!({
                        "error": { "message": error.to_string(), "type": "server_error" }
                    })),
                    ChunkState::Done,
                ),
            },
            ChunkState::Usage(usage) => (header.event(Vec::new(), Some(usage)), ChunkState::Done),
            ChunkState::Done => (Ok(Event::default().data("[DONE]")), ChunkState::Ended),
            ChunkState::Ended => return None,
        };
        Some((event, (header, next)))
    })
}
