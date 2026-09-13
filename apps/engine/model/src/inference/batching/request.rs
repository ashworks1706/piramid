//! What a generation streams back to its caller.

use std::time::Duration;

use piramid_core::error::InferenceError;

/// Why a generation ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FinishReason {
    /// An end-of-sequence token or a stop string.
    Stop,
    /// max_new_tokens or the sequence length limit.
    Length,
}

impl FinishReason {
    /// The name OpenAI-compatible clients expect.
    pub fn as_str(&self) -> &'static str {
        match self {
            FinishReason::Stop => "stop",
            FinishReason::Length => "length",
        }
    }
}

/// Token counts and timings of one generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Usage {
    /// Tokens in the prompt.
    pub prompt_tokens: usize,
    /// Prompt tokens served from shared cache pages.
    pub cached_prompt_tokens: usize,
    /// Tokens generated.
    pub completion_tokens: usize,
    /// From admission to the first generated token.
    pub time_to_first_token: Option<Duration>,
    /// From admission to the end.
    pub total_time: Duration,
}

/// One event of a streamed generation. Finished or Failed is always the last event.
#[derive(Debug, Clone, PartialEq)]
pub enum GenerationEvent {
    /// A generated token and the text it completes, which may be empty.
    Token {
        /// The token id.
        token: u32,
        /// Text released by this token.
        text: String,
    },
    /// The generation ended normally.
    Finished {
        /// Why it ended.
        reason: FinishReason,
        /// Counts and timings.
        usage: Usage,
    },
    /// The generation ended with an error.
    Failed(InferenceError),
}
