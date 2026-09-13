//! Generation endpoints: /api/generate, /api/model, and the OpenAI-compatible
//! /v1/chat/completions and /v1/models.

use std::convert::Infallible;

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
    OpenAiModel, OpenAiModelList, OpenAiResponseMessage, TokenEventDto,
};
use crate::services::generation;
use crate::state::SharedState;

/// GET /api/model: the loaded model.
pub async fn model(State(state): State<SharedState>) -> Result<Json<ModelResponse>> {
    Ok(Json(generation::model(&state)?))
}

/// POST /api/generate: generate from a prompt or conversation, optionally after retrieval, as one
/// JSON body or as server-sent events.
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

fn native_events(
    generation: Generation,
) -> impl Stream<Item = std::result::Result<Event, Infallible>> {
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

fn json_event<T: serde::Serialize>(name: &str, body: &T) -> std::result::Result<Event, Infallible> {
    Ok(Event::default()
        .event(name)
        .data(serde_json::to_string(body).unwrap_or_else(|e| format!("{{\"error\":\"{e}\"}}"))))
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
    let created = piramid_core::clock::unix_secs();
    if stream_chunks {
        let chunks = openai_chunks(generation, id, model, created, include_usage);
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

enum ChunkState {
    Streaming { generation: Generation, first: bool },
    Usage(ChatCompletionChunk),
    Done,
    Ended,
}

fn openai_chunks(
    generation: Generation,
    id: String,
    model: String,
    created: u64,
    include_usage: bool,
) -> impl Stream<Item = std::result::Result<Event, Infallible>> {
    let chunk = move |choices: Vec<OpenAiChunkChoice>, usage| ChatCompletionChunk {
        id: id.clone(),
        object: "chat.completion.chunk",
        created,
        model: model.clone(),
        choices,
        usage,
    };
    stream::unfold(
        ChunkState::Streaming {
            generation,
            first: true,
        },
        move |state| {
            let chunk = chunk.clone();
            async move {
                match state {
                    ChunkState::Streaming {
                        mut generation,
                        first,
                    } => {
                        let event = generation.next().await?;
                        match event {
                            GenerationEvent::Token { text, .. } => {
                                let body = chunk(
                                    vec![OpenAiChunkChoice {
                                        index: 0,
                                        delta: OpenAiDelta {
                                            role: first.then_some("assistant"),
                                            content: Some(text),
                                        },
                                        finish_reason: None,
                                    }],
                                    None,
                                );
                                Some((
                                    data_event(&body),
                                    ChunkState::Streaming {
                                        generation,
                                        first: false,
                                    },
                                ))
                            }
                            GenerationEvent::Finished { reason, usage } => {
                                let body = chunk(
                                    vec![OpenAiChunkChoice {
                                        index: 0,
                                        delta: OpenAiDelta {
                                            role: first.then_some("assistant"),
                                            content: None,
                                        },
                                        finish_reason: Some(reason.as_str()),
                                    }],
                                    None,
                                );
                                let next = if include_usage {
                                    ChunkState::Usage(chunk(
                                        Vec::new(),
                                        Some(generation::openai_usage(&usage)),
                                    ))
                                } else {
                                    ChunkState::Done
                                };
                                Some((data_event(&body), next))
                            }
                            GenerationEvent::Failed(error) => Some((
                                Ok(Event::default().data(
                                    serde_json::json!({
                                        "error": { "message": error.to_string(), "type": "server_error" }
                                    })
                                    .to_string(),
                                )),
                                ChunkState::Done,
                            )),
                        }
                    }
                    ChunkState::Usage(body) => Some((data_event(&body), ChunkState::Done)),
                    ChunkState::Done => {
                        Some((Ok(Event::default().data("[DONE]")), ChunkState::Ended))
                    }
                    ChunkState::Ended => None,
                }
            }
        },
    )
}

fn data_event<T: serde::Serialize>(body: &T) -> std::result::Result<Event, Infallible> {
    Ok(Event::default()
        .data(serde_json::to_string(body).unwrap_or_else(|e| format!("{{\"error\":\"{e}\"}}"))))
}
