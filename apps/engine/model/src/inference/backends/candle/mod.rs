//! Candle-backed model execution. Compiled only under inference-candle.

pub mod qwen;
pub mod runtime;
pub mod weights;

pub use qwen::QwenModel;
pub use runtime::CandleRuntime;
