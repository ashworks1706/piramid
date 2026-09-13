//! Request admission and batch assembly: sequences wait in a queue, a scheduler packs decode
//! tokens and prefill chunks into each forward step under the batch and cache limits, and a worker
//! thread runs the steps and streams tokens back.

pub mod request;
pub mod scheduler;
pub mod stop;
pub mod worker;

pub use request::{FinishReason, GenerationEvent, Usage};
pub use scheduler::{PlannedStep, Scheduler, SchedulerLimits, Sequence};
pub use stop::StopMatcher;
