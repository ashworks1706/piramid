//! Model runtime backends; the only place candle or tokenizers types may appear.

#[cfg(feature = "inference-candle")]
pub mod candle;
#[cfg(feature = "inference-candle")]
pub mod tokenizers;
