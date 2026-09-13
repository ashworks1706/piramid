//! The retrieval seam: the trait a forward-pass driver calls into.

use piramid_core::error::Result;
use piramid_hardware::gpu::{DeviceBuffer, Stream};

/// Where in the forward pass a hook is being invoked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetrievalPoint {
    /// Before the first decoder layer, once per sequence.
    SequenceStart,
    /// Chunk boundary during decoding.
    ChunkBoundary {
        /// Zero-based index of the chunk that just ended.
        chunk: usize,
    },
    /// Before a specific decoder layer.
    LayerEntry {
        /// Zero-based decoder layer index.
        layer: usize,
    },
}

/// Where the hidden state for the current position lives.
///
/// A hook that implements only one of the two paths returns an error on the other.
#[derive(Debug)]
pub enum HiddenState<'a> {
    /// Host memory, laid out as batch by hidden_dim. The CPU path.
    Host(&'a mut [f32]),
    /// Device memory, same layout. Fusing into this must stay on the device.
    Device(&'a mut DeviceBuffer<f32>),
}

/// Read-only view handed to [RetrievalHook::launch]. Carries enough to build a query, and no
/// access to the hidden state.
#[derive(Debug)]
pub struct RetrievalRequest<'a> {
    /// Where in the pass this invocation sits.
    pub point: RetrievalPoint,
    /// Tokens generated so far, for building a retrieval query.
    pub tokens: &'a [u32],
    /// Width of the hidden dimension.
    pub hidden_dim: usize,
    /// The stream that model work is queued on, when running on a device. A hook that issues
    /// device work uses a stream of its own, and orders against this one in
    /// [PendingRetrieval::join].
    pub stream: Option<&'a Stream>,
}

/// Mutable view of the forward pass, handed to [PendingRetrieval::join] when the result is
/// needed.
#[derive(Debug)]
pub struct ForwardContext<'a> {
    /// Where in the pass this invocation sits.
    pub point: RetrievalPoint,
    /// Hidden state for the current position.
    pub hidden: HiddenState<'a>,
    /// Width of the hidden dimension.
    pub hidden_dim: usize,
    /// The stream that model work is queued on, when running on a device.
    pub stream: Option<&'a Stream>,
}

/// Retrieval that has been started and not yet fused.
///
/// Held by the driver across whatever model work it can do in the meantime.
pub trait PendingRetrieval: Send {
    /// Wait for the result and fuse it into the hidden state carried by ctx.
    ///
    /// On a device this orders the model stream against the hook stream without synchronizing the
    /// host.
    fn join(self: Box<Self>, ctx: &mut ForwardContext<'_>) -> Result<()>;
}

/// A retrieval strategy that participates in the forward pass.
pub trait RetrievalHook: Send + Sync {
    /// Name for logs and configuration.
    fn name(&self) -> &'static str;

    /// Whether this hook runs at the given point.
    fn wants(&self, point: RetrievalPoint) -> bool;

    /// Start retrieval. Must return without waiting for the result.
    fn launch(&self, request: &RetrievalRequest<'_>) -> Result<Box<dyn PendingRetrieval>>;
}

/// A hook that never fires; the default and the control arm for fusion benchmarks.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopRetrievalHook;

impl RetrievalHook for NoopRetrievalHook {
    fn name(&self) -> &'static str {
        "noop"
    }

    fn wants(&self, _point: RetrievalPoint) -> bool {
        false
    }

    fn launch(&self, _request: &RetrievalRequest<'_>) -> Result<Box<dyn PendingRetrieval>> {
        Ok(Box::new(NoopPending))
    }
}

/// The pending half of [NoopRetrievalHook]; joining it leaves the pass untouched.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopPending;

impl PendingRetrieval for NoopPending {
    fn join(self: Box<Self>, _ctx: &mut ForwardContext<'_>) -> Result<()> {
        Ok(())
    }
}
