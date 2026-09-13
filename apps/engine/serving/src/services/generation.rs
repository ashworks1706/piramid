//! Generation use cases: building the prompt, retrieving passages before prefill, merging
//! sampling overrides, and turning engine events into API shapes.

use std::sync::Arc;
use std::time::Instant;

use piramid_core::config::SamplingConfig;
use piramid_core::error::{Result, ServerError};
use piramid_database::search::SearchParams;
use piramid_model::inference::batching::Usage;
use piramid_model::inference::manager::Generation;
use piramid_model::inference::tokenizer::ChatMessage;
use piramid_model::inference::InferenceManager;

use crate::services::api::{
    ChatCompletionRequest, GenerateRequest, GenerationUsageDto, MessageDto, ModelResponse,
    OpenAiContent, OpenAiStop, OpenAiUsage, PassageDto, RetrievalDto, RetrievedDto,
};
use crate::state::SharedState;

/// Error message for a generation request when no model is loaded.
pub const INFERENCE_NOT_ENABLED: &str =
    "no model is loaded; set runtime.inference.enabled with a model_path and restart";

/// The loaded model, or 503 when there is none.
pub fn manager(state: &SharedState) -> Result<Arc<InferenceManager>> {
    state.ensure_available()?;
    state
        .inference
        .clone()
        .ok_or_else(|| ServerError::ServiceUnavailable(INFERENCE_NOT_ENABLED.to_string()).into())
}

/// What is loaded.
pub fn model(state: &SharedState) -> Result<ModelResponse> {
    let info = manager(state)?.info();
    Ok(ModelResponse {
        name: info.name,
        architecture: info.architecture,
        device: info.device,
        max_sequence_length: info.max_sequence_length,
        hook: info.hook,
    })
}

/// A generation admitted by the engine, and what retrieval put before it.
#[derive(Debug)]
pub struct Started {
    /// The streamed generation.
    pub generation: Generation,
    /// Retrieval results, when retrieval ran.
    pub retrieval: Option<RetrievedDto>,
}

/// Build the prompt of a /api/generate request, retrieve if asked, and queue the generation.
pub async fn start(state: &SharedState, request: GenerateRequest) -> Result<Started> {
    let manager = manager(state)?;
    let mut sampling = manager.defaults().clone();
    apply_overrides(
        &mut sampling,
        Overrides {
            max_new_tokens: request.max_new_tokens,
            temperature: request.temperature,
            top_p: request.top_p,
            top_k: request.top_k,
            repetition_penalty: request.repetition_penalty,
            seed: request.seed,
            stop: request.stop,
        },
    );

    let (prompt, retrieval) = match (request.prompt, request.messages) {
        (Some(prompt), None) => {
            let retrieval = match &request.retrieval {
                Some(retrieval) => Some(retrieve(state, retrieval, &prompt).await?),
                None => None,
            };
            let text = match &retrieval {
                Some(found) => format!("{}{prompt}", passages_block(&found.passages)),
                None => prompt,
            };
            (text, retrieval)
        }
        (None, Some(messages)) => {
            let mut messages: Vec<ChatMessage> = messages.into_iter().map(to_chat).collect();
            let retrieval = match &request.retrieval {
                Some(retrieval) => {
                    let query = last_user_message(&messages)?;
                    Some(retrieve(state, retrieval, &query).await?)
                }
                None => None,
            };
            if let Some(found) = &retrieval {
                insert_passages(&mut messages, &found.passages);
            }
            (manager.render_chat(&messages)?, retrieval)
        }
        (Some(_), Some(_)) | (None, None) => {
            return Err(ServerError::InvalidRequest(
                "give exactly one of prompt and messages".to_string(),
            )
            .into())
        }
    };
    let generation = queue(&manager, &prompt, sampling).await?;
    Ok(Started {
        generation,
        retrieval,
    })
}

/// Queue an OpenAI chat completion, returning the model id it answers as.
pub async fn start_chat(
    state: &SharedState,
    request: ChatCompletionRequest,
) -> Result<(String, Generation)> {
    let manager = manager(state)?;
    let model_id = manager.info().name;
    if request.model != model_id {
        return Err(ServerError::NotFound(format!(
            "model {} is not loaded; this server serves {model_id}",
            request.model
        ))
        .into());
    }
    let unsupported = |what: &str| -> Result<(String, Generation)> {
        Err(ServerError::InvalidRequest(format!("{what} is not supported")).into())
    };
    if request.n.is_some_and(|n| n != 1) {
        return unsupported("n other than 1");
    }
    if request.frequency_penalty.is_some_and(|p| p != 0.0) {
        return unsupported("frequency_penalty");
    }
    if request.presence_penalty.is_some_and(|p| p != 0.0) {
        return unsupported("presence_penalty");
    }
    if request.max_tokens.is_some() && request.max_completion_tokens.is_some() {
        return Err(ServerError::InvalidRequest(
            "give max_tokens or max_completion_tokens, not both".to_string(),
        )
        .into());
    }
    let mut messages = Vec::with_capacity(request.messages.len());
    for message in request.messages {
        if message.name.is_some() {
            return unsupported("a message name");
        }
        let content = match message.content {
            OpenAiContent::Text(text) => text,
            OpenAiContent::Parts(parts) => {
                let mut text = String::new();
                for part in parts {
                    match (part.kind.as_str(), part.text) {
                        ("text", Some(part)) => text.push_str(&part),
                        (kind, _) => return unsupported(&format!("content part {kind}")),
                    }
                }
                text
            }
        };
        messages.push(to_chat(MessageDto {
            role: message.role,
            content,
        }));
    }
    let mut sampling = manager.defaults().clone();
    apply_overrides(
        &mut sampling,
        Overrides {
            max_new_tokens: request.max_tokens.or(request.max_completion_tokens),
            temperature: request.temperature,
            top_p: request.top_p,
            top_k: None,
            repetition_penalty: None,
            seed: request.seed,
            stop: request.stop.map(|stop| match stop {
                OpenAiStop::One(stop) => vec![stop],
                OpenAiStop::Many(stops) => stops,
            }),
        },
    );
    let prompt = manager.render_chat(&messages)?;
    let generation = queue(&manager, &prompt, sampling).await?;
    Ok((model_id, generation))
}

async fn queue(
    manager: &InferenceManager,
    prompt: &str,
    sampling: SamplingConfig,
) -> Result<Generation> {
    let tokens = manager.tokenize(prompt)?;
    Ok(manager.generate(tokens, sampling).await?)
}

struct Overrides {
    max_new_tokens: Option<usize>,
    temperature: Option<f32>,
    top_p: Option<f32>,
    top_k: Option<usize>,
    repetition_penalty: Option<f32>,
    seed: Option<u64>,
    stop: Option<Vec<String>>,
}

fn apply_overrides(sampling: &mut SamplingConfig, overrides: Overrides) {
    if let Some(value) = overrides.max_new_tokens {
        sampling.max_new_tokens = value;
    }
    if let Some(value) = overrides.temperature {
        sampling.temperature = value;
    }
    if overrides.top_p.is_some() {
        sampling.top_p = overrides.top_p;
    }
    if overrides.top_k.is_some() {
        sampling.top_k = overrides.top_k;
    }
    if let Some(value) = overrides.repetition_penalty {
        sampling.repetition_penalty = value;
    }
    if overrides.seed.is_some() {
        sampling.seed = overrides.seed;
    }
    if let Some(stop) = overrides.stop {
        sampling.stop = stop;
    }
}

fn to_chat(message: MessageDto) -> ChatMessage {
    ChatMessage {
        role: message.role,
        content: message.content,
    }
}

fn last_user_message(messages: &[ChatMessage]) -> Result<String> {
    messages
        .iter()
        .rev()
        .find(|message| message.role == "user")
        .map(|message| message.content.clone())
        .ok_or_else(|| {
            ServerError::InvalidRequest(
                "retrieval needs a query or a user message to search with".to_string(),
            )
            .into()
        })
}

/// Passages as a numbered block placed before a prompt.
pub fn passages_block(passages: &[PassageDto]) -> String {
    let mut block = String::new();
    for (index, passage) in passages.iter().enumerate() {
        block.push_str(&format!("[{}] {}\n\n", index + 1, passage.text));
    }
    block
}

/// Put passages in the system message, creating one at the front when there is none.
fn insert_passages(messages: &mut Vec<ChatMessage>, passages: &[PassageDto]) {
    let block = format!(
        "Answer using these passages where they are relevant.\n\n{}",
        passages_block(passages)
    );
    match messages.first_mut() {
        Some(first) if first.role == "system" => {
            first.content = format!("{}\n\n{}", first.content, block.trim_end());
        }
        _ => messages.insert(
            0,
            ChatMessage {
                role: "system".to_string(),
                content: block.trim_end().to_string(),
            },
        ),
    }
}

async fn retrieve(
    state: &SharedState,
    retrieval: &RetrievalDto,
    fallback_query: &str,
) -> Result<RetrievedDto> {
    if retrieval.k == 0 {
        return Err(ServerError::InvalidRequest("retrieval.k must be >= 1".to_string()).into());
    }
    let handle = state.get_existing_collection(&retrieval.collection)?;
    let embedder = state.embeddings.embedder().ok_or_else(|| {
        ServerError::ServiceUnavailable(crate::services::EMBEDDING_NOT_CONFIGURED.to_string())
    })?;
    let query = retrieval.query.as_deref().unwrap_or(fallback_query);

    let started = Instant::now();
    let embedded = embedder.embed(query).await?;
    let embed_elapsed = started.elapsed();
    state
        .embeddings
        .metrics()
        .record(1, 1, u64::from(embedded.tokens.unwrap_or(0)), embed_elapsed);

    let started = Instant::now();
    let hits = {
        let guard = handle.read();
        let metric = guard.vector_index().metric();
        guard.search(
            &embedded.embedding,
            retrieval.k,
            metric,
            SearchParams {
                mode: guard.config().execution,
                ..SearchParams::default()
            },
        )?
    };
    let search_elapsed = started.elapsed();
    Ok(RetrievedDto {
        collection: retrieval.collection.clone(),
        passages: hits
            .into_iter()
            .map(|hit| PassageDto {
                id: hit.document.id.to_string(),
                score: hit.score,
                text: hit.document.text,
            })
            .collect(),
        embed_ms: embed_elapsed.as_secs_f64() * 1e3,
        search_ms: search_elapsed.as_secs_f64() * 1e3,
    })
}

/// Usage in native API form.
pub fn usage(usage: &Usage) -> GenerationUsageDto {
    GenerationUsageDto {
        prompt_tokens: usage.prompt_tokens,
        cached_prompt_tokens: usage.cached_prompt_tokens,
        completion_tokens: usage.completion_tokens,
        time_to_first_token_ms: usage.time_to_first_token.map(|d| d.as_secs_f64() * 1e3),
        total_ms: usage.total_time.as_secs_f64() * 1e3,
    }
}

/// Usage in OpenAI form.
pub fn openai_usage(usage: &Usage) -> OpenAiUsage {
    OpenAiUsage {
        prompt_tokens: usage.prompt_tokens,
        completion_tokens: usage.completion_tokens,
        total_tokens: usage.prompt_tokens + usage.completion_tokens,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
