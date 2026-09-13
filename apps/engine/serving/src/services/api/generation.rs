//! Generation request and response shapes: the native /api/generate endpoint and the
//! OpenAI-compatible chat completion endpoint.

use serde::{Deserialize, Serialize};

/// One turn of a conversation.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MessageDto {
    /// system, user, assistant or tool.
    pub role: String,
    /// What the turn says.
    pub content: String,
}

/// Retrieve passages from a collection and place them before the prompt.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RetrievalDto {
    /// Collection to search. It must hold text embedded by the configured provider.
    pub collection: String,
    /// Passages to retrieve. 4 when omitted.
    #[serde(default = "default_passages")]
    pub k: usize,
    /// Text to embed as the query. The prompt, or the last user message, when omitted.
    pub query: Option<String>,
}

fn default_passages() -> usize {
    4
}

/// POST /api/generate. Exactly one of prompt and messages is given.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GenerateRequest {
    /// Prompt text passed to the model as it stands.
    pub prompt: Option<String>,
    /// A conversation rendered through the model's chat template.
    pub messages: Option<Vec<MessageDto>>,
    /// Passages retrieved before prefill.
    pub retrieval: Option<RetrievalDto>,
    /// Stream server-sent events instead of returning one JSON body.
    #[serde(default)]
    pub stream: bool,
    /// Tokens to generate at most. The configured default when omitted.
    pub max_new_tokens: Option<usize>,
    /// Draw temperature; 0 is greedy.
    pub temperature: Option<f32>,
    /// Nucleus cutoff.
    pub top_p: Option<f32>,
    /// Keep only this many highest-probability tokens.
    pub top_k: Option<usize>,
    /// Penalty on recently generated tokens; 1 is none.
    pub repetition_penalty: Option<f32>,
    /// Fixed seed for the sampler.
    pub seed: Option<u64>,
    /// Strings that end the generation.
    pub stop: Option<Vec<String>>,
}

/// A passage placed before the prompt.
#[derive(Debug, Clone, Serialize)]
pub struct PassageDto {
    /// Document id.
    pub id: String,
    /// Similarity score.
    pub score: f32,
    /// Passage text.
    pub text: String,
}

/// What retrieval found and how long it took.
#[derive(Debug, Clone, Serialize)]
pub struct RetrievedDto {
    /// Collection searched.
    pub collection: String,
    /// Passages in rank order.
    pub passages: Vec<PassageDto>,
    /// Time embedding the query, in milliseconds.
    pub embed_ms: f64,
    /// Time searching, in milliseconds.
    pub search_ms: f64,
}

/// Token counts and timings of one generation.
#[derive(Debug, Clone, Serialize)]
pub struct GenerationUsageDto {
    /// Tokens in the prompt, retrieved passages included.
    pub prompt_tokens: usize,
    /// Prompt tokens served from shared cache pages.
    pub cached_prompt_tokens: usize,
    /// Tokens generated.
    pub completion_tokens: usize,
    /// From admission to the first generated token, in milliseconds. Absent when no token was
    /// generated.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_to_first_token_ms: Option<f64>,
    /// From admission to the end, in milliseconds.
    pub total_ms: f64,
}

/// The body of a non-streamed /api/generate response.
#[derive(Debug, Clone, Serialize)]
pub struct GenerateResponse {
    /// Generated text.
    pub text: String,
    /// stop or length.
    pub finish_reason: &'static str,
    /// Counts and timings.
    pub usage: GenerationUsageDto,
    /// Retrieval results, when the request asked for retrieval.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retrieval: Option<RetrievedDto>,
}

/// A streamed token event.
#[derive(Debug, Clone, Serialize)]
pub struct TokenEventDto {
    /// Token id.
    pub token: u32,
    /// Text released by the token, possibly empty.
    pub text: String,
}

/// The final event of a streamed generation.
#[derive(Debug, Clone, Serialize)]
pub struct DoneEventDto {
    /// stop or length.
    pub finish_reason: &'static str,
    /// Counts and timings.
    pub usage: GenerationUsageDto,
}

/// GET /api/model: what is loaded.
#[derive(Debug, Clone, Serialize)]
pub struct ModelResponse {
    /// Name clients address the model by, also the OpenAI model id.
    pub name: String,
    /// Decoder family.
    pub architecture: &'static str,
    /// Device the model runs on.
    pub device: String,
    /// Longest prompt plus completion.
    pub max_sequence_length: usize,
    /// Retrieval hook the forward pass consults.
    pub hook: &'static str,
}

/// Content of an OpenAI message: a string, or a list of text parts.
#[derive(Debug, Clone)]
pub enum OpenAiContent {
    /// Plain text.
    Text(String),
    /// Content parts; only text parts are accepted.
    Parts(Vec<OpenAiContentPart>),
}

impl<'de> Deserialize<'de> for OpenAiContent {
    /// Reads a string as text and a list as content parts, keeping the error of a part that fails.
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de::Error;
        match serde_json::Value::deserialize(deserializer)? {
            serde_json::Value::String(text) => Ok(Self::Text(text)),
            parts @ serde_json::Value::Array(_) => serde_json::from_value(parts)
                .map(Self::Parts)
                .map_err(D::Error::custom),
            _ => Err(D::Error::custom(
                "content must be a string or a list of content parts",
            )),
        }
    }
}

/// One content part of an OpenAI message.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OpenAiContentPart {
    /// Part type; only text is accepted.
    #[serde(rename = "type")]
    pub kind: String,
    /// Text of a text part.
    pub text: Option<String>,
}

/// One OpenAI chat message.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OpenAiMessage {
    /// system, user, assistant or tool.
    pub role: String,
    /// What the turn says.
    pub content: OpenAiContent,
    /// Author name; not supported.
    pub name: Option<String>,
}

/// Stop sequences as a string or a list.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum OpenAiStop {
    /// One stop string.
    One(String),
    /// Several stop strings.
    Many(Vec<String>),
}

/// Streaming options.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OpenAiStreamOptions {
    /// Send a final chunk carrying usage.
    #[serde(default)]
    pub include_usage: bool,
}

/// POST /v1/chat/completions.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChatCompletionRequest {
    /// Model id; must name the loaded model.
    pub model: String,
    /// The conversation.
    pub messages: Vec<OpenAiMessage>,
    /// Stream chunks as server-sent events.
    #[serde(default)]
    pub stream: bool,
    /// Streaming options.
    pub stream_options: Option<OpenAiStreamOptions>,
    /// Tokens to generate at most.
    pub max_tokens: Option<usize>,
    /// Tokens to generate at most; the newer spelling of max_tokens. Giving both is refused.
    pub max_completion_tokens: Option<usize>,
    /// Draw temperature.
    pub temperature: Option<f32>,
    /// Nucleus cutoff.
    pub top_p: Option<f32>,
    /// Stop sequences.
    pub stop: Option<OpenAiStop>,
    /// Fixed seed.
    pub seed: Option<u64>,
    /// Completions per request; only 1 is served.
    pub n: Option<usize>,
    /// Not supported unless 0.
    pub frequency_penalty: Option<f32>,
    /// Not supported unless 0.
    pub presence_penalty: Option<f32>,
    /// End-user identifier; not supported.
    pub user: Option<String>,
}

/// A message in a chat completion response.
#[derive(Debug, Clone, Serialize)]
pub struct OpenAiResponseMessage {
    /// Always assistant.
    pub role: &'static str,
    /// Generated text.
    pub content: String,
}

/// One choice of a chat completion.
#[derive(Debug, Clone, Serialize)]
pub struct OpenAiChoice {
    /// Always 0.
    pub index: usize,
    /// The generated message.
    pub message: OpenAiResponseMessage,
    /// stop or length.
    pub finish_reason: &'static str,
}

/// Token counts in OpenAI form.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct OpenAiUsage {
    /// Prompt tokens.
    pub prompt_tokens: usize,
    /// Generated tokens.
    pub completion_tokens: usize,
    /// Their sum.
    pub total_tokens: usize,
}

/// A chat completion response.
#[derive(Debug, Clone, Serialize)]
pub struct ChatCompletionResponse {
    /// Completion id.
    pub id: String,
    /// Always chat.completion.
    pub object: &'static str,
    /// Creation time, in seconds since the Unix epoch.
    pub created: u64,
    /// Model id.
    pub model: String,
    /// The single choice.
    pub choices: Vec<OpenAiChoice>,
    /// Token counts.
    pub usage: OpenAiUsage,
}

/// The incremental message of a streamed chunk.
#[derive(Debug, Clone, Serialize, Default)]
pub struct OpenAiDelta {
    /// assistant on the first chunk.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<&'static str>,
    /// New text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
}

/// One choice of a streamed chunk.
#[derive(Debug, Clone, Serialize)]
pub struct OpenAiChunkChoice {
    /// Always 0.
    pub index: usize,
    /// The increment.
    pub delta: OpenAiDelta,
    /// Set on the last content chunk.
    pub finish_reason: Option<&'static str>,
}

/// A streamed chat completion chunk.
#[derive(Debug, Clone, Serialize)]
pub struct ChatCompletionChunk<'a> {
    /// Completion id, the same on every chunk.
    pub id: &'a str,
    /// Always chat.completion.chunk.
    pub object: &'static str,
    /// Creation time, in seconds since the Unix epoch.
    pub created: u64,
    /// Model id.
    pub model: &'a str,
    /// Empty on a usage chunk.
    pub choices: Vec<OpenAiChunkChoice>,
    /// Token counts, only on the usage chunk.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<OpenAiUsage>,
}

/// One entry of GET /v1/models.
#[derive(Debug, Clone, Serialize)]
pub struct OpenAiModel {
    /// Model id.
    pub id: String,
    /// Always model.
    pub object: &'static str,
    /// Load time, in seconds since the Unix epoch.
    pub created: u64,
    /// Always piramid.
    pub owned_by: &'static str,
}

/// GET /v1/models.
#[derive(Debug, Clone, Serialize)]
pub struct OpenAiModelList {
    /// Always list.
    pub object: &'static str,
    /// The loaded model.
    pub data: Vec<OpenAiModel>,
}
