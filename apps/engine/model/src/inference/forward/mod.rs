//! The forward-pass driver: runs a [DecoderModel] one layer at a time over a step batch and calls
//! the retrieval hook at every point it asks for.

use std::sync::Arc;

use piramid_core::error::InferenceError;

use crate::fusion::{ForwardContext, RetrievalHook, RetrievalPoint, RetrievalRequest};
use crate::inference::architecture::{DecoderModel, StepBatch};

/// Where one sequence of a step stands, for choosing the hook points it passes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SequenceProgress<'a> {
    /// Every token of the sequence so far, prompt and generated.
    pub tokens: &'a [u32],
    /// Whether this step computes the sequence's first token.
    pub first_step: bool,
    /// The index of a generated chunk that ended just before this step, if one did.
    pub finished_chunk: Option<usize>,
}

/// Drives one loaded model through forward steps.
pub struct Driver<M: DecoderModel> {
    model: M,
    hook: Arc<dyn RetrievalHook>,
}

impl<M: DecoderModel> std::fmt::Debug for Driver<M> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Driver")
            .field("hook", &self.hook.name())
            .finish_non_exhaustive()
    }
}

impl<M: DecoderModel> Driver<M> {
    /// A driver over model that consults hook.
    pub fn new(model: M, hook: Arc<dyn RetrievalHook>) -> Self {
        Self { model, hook }
    }

    /// The model this driver runs.
    pub fn model(&self) -> &M {
        &self.model
    }

    /// The model this driver runs, mutably.
    pub fn model_mut(&mut self) -> &mut M {
        &mut self.model
    }

    /// Run one step. progress holds one entry per batch sequence, in batch order. Returns the
    /// last-token logits of every sequence that asked for them, in batch order.
    pub fn step(
        &mut self,
        batch: &StepBatch,
        progress: &[SequenceProgress<'_>],
    ) -> Result<Vec<Vec<f32>>, InferenceError> {
        if progress.len() != batch.sequences.len() {
            return Err(InferenceError::Runtime(format!(
                "{} progress entries for {} sequences",
                progress.len(),
                batch.sequences.len()
            )));
        }
        let mut pass = self.model.begin(batch)?;
        for (sequence, entry) in progress.iter().enumerate() {
            if entry.first_step {
                self.retrieve(&mut pass, sequence, entry, RetrievalPoint::SequenceStart)?;
            }
            if let Some(chunk) = entry.finished_chunk {
                self.retrieve(
                    &mut pass,
                    sequence,
                    entry,
                    RetrievalPoint::ChunkBoundary { chunk },
                )?;
            }
        }
        for layer in 0..self.model.spec().layers {
            for (sequence, entry) in progress.iter().enumerate() {
                self.retrieve(
                    &mut pass,
                    sequence,
                    entry,
                    RetrievalPoint::LayerEntry { layer },
                )?;
            }
            self.model.layer(&mut pass, layer)?;
        }
        self.model.finish(pass)
    }

    fn retrieve(
        &mut self,
        pass: &mut M::Pass,
        sequence: usize,
        progress: &SequenceProgress<'_>,
        point: RetrievalPoint,
    ) -> Result<(), InferenceError> {
        if !self.hook.wants(point) {
            return Ok(());
        }
        let hidden_dim = self.model.spec().hidden_size;
        let name = self.hook.name();
        let pending = self
            .hook
            .launch(&RetrievalRequest {
                point,
                tokens: progress.tokens,
                hidden_dim,
                stream: self.model.stream(),
            })
            .map_err(|e| InferenceError::Runtime(format!("{name} launch: {e}")))?;
        let mut pending = Some(pending);
        self.model
            .with_hidden(pass, sequence, &mut |hidden, stream| {
                let Some(pending) = pending.take() else {
                    return Err(InferenceError::Runtime(format!("{name} joined twice")));
                };
                let mut context = ForwardContext {
                    point,
                    hidden,
                    hidden_dim,
                    stream,
                };
                pending
                    .join(&mut context)
                    .map_err(|e| InferenceError::Runtime(format!("{name} join: {e}")))
            })
    }
}
