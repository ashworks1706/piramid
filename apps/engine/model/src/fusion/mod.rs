//! The seam where retrieval enters the forward pass.

mod hook;

pub use hook::{
    ForwardContext, HiddenState, NoopPending, NoopRetrievalHook, PendingRetrieval, RetrievalHook,
    RetrievalPoint, RetrievalRequest,
};
