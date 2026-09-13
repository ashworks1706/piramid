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
        let pending = self
            .hook
            .launch(&RetrievalRequest {
                point,
                tokens: progress.tokens,
                hidden_dim,
                stream: None,
            })
            .map_err(|e| InferenceError::Runtime(format!("{} launch: {e}", self.hook.name())))?;
        let name = self.hook.name();
        let mut pending = Some(pending);
        self.model.with_hidden(pass, sequence, &mut |hidden| {
            let Some(pending) = pending.take() else {
                return Err(InferenceError::Runtime(format!("{name} joined twice")));
            };
            let mut context = ForwardContext {
                point,
                hidden,
                hidden_dim,
                stream: None,
            };
            pending
                .join(&mut context)
                .map_err(|e| InferenceError::Runtime(format!("{name} join: {e}")))
        })
    }
}

#[cfg(all(test, feature = "inference-candle"))]
mod tests {
    #![allow(clippy::unwrap_used, reason = "assertions in tests")]

    use std::sync::Mutex;

    use piramid_core::error::Result as CoreResult;

    use super::*;
    use crate::fusion::{HiddenState, NoopRetrievalHook, PendingRetrieval};
    use crate::inference::architecture::{Architecture, StepSequence};
    use crate::inference::backends::candle::qwen::testing::tiny_model;

    fn prefill(tokens: &[u32], start: usize, slot_base: u32, logits: bool) -> StepSequence {
        let end = start + tokens.len();
        StepSequence {
            tokens: tokens.to_vec(),
            start,
            write_slots: (start..end).map(|p| slot_base + p as u32).collect(),
            context_slots: (0..end).map(|p| slot_base + p as u32).collect(),
            logits,
        }
    }

    fn progress(tokens: &[u32], first_step: bool) -> SequenceProgress<'_> {
        SequenceProgress {
            tokens,
            first_step,
            finished_chunk: None,
        }
    }

    fn close(a: &[f32], b: &[f32]) -> bool {
        a.len() == b.len() && a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-4)
    }

    #[test]
    fn decoding_from_the_cache_matches_recomputing_the_whole_sequence() {
        for architecture in [Architecture::Qwen2, Architecture::Qwen3] {
            let tokens = [3u32, 14, 15, 92, 65, 35];
            let hook: Arc<dyn RetrievalHook> = Arc::new(NoopRetrievalHook);

            let mut whole = Driver::new(tiny_model(architecture, 1, 64), hook.clone());
            let batch = StepBatch {
                sequences: vec![prefill(&tokens, 0, 0, true)],
            };
            let expected = whole.step(&batch, &[progress(&tokens, true)]).unwrap();

            let mut incremental = Driver::new(tiny_model(architecture, 1, 64), hook);
            let batch = StepBatch {
                sequences: vec![prefill(&tokens[..4], 0, 8, false)],
            };
            assert!(incremental
                .step(&batch, &[progress(&tokens[..4], true)])
                .unwrap()
                .is_empty());
            let mut got = Vec::new();
            for position in 4..6 {
                let batch = StepBatch {
                    sequences: vec![prefill(&tokens[position..=position], position, 8, true)],
                };
                got = incremental
                    .step(&batch, &[progress(&tokens[..=position], false)])
                    .unwrap();
            }
            assert!(close(&got[0], &expected[0]), "{architecture:?}");
        }
    }

    #[test]
    fn a_batched_step_matches_each_sequence_alone() {
        let hook: Arc<dyn RetrievalHook> = Arc::new(NoopRetrievalHook);
        let first = [5u32, 6, 7];
        let second = [40u32, 41, 42, 43, 44];

        let mut alone = Driver::new(tiny_model(Architecture::Qwen3, 2, 64), hook.clone());
        let a = alone
            .step(
                &StepBatch {
                    sequences: vec![prefill(&first, 0, 0, true)],
                },
                &[progress(&first, true)],
            )
            .unwrap();
        let b = alone
            .step(
                &StepBatch {
                    sequences: vec![prefill(&second, 0, 16, true)],
                },
                &[progress(&second, true)],
            )
            .unwrap();

        let mut batched = Driver::new(tiny_model(Architecture::Qwen3, 2, 64), hook);
        let both = batched
            .step(
                &StepBatch {
                    sequences: vec![prefill(&first, 0, 0, true), prefill(&second, 0, 16, true)],
                },
                &[progress(&first, true), progress(&second, true)],
            )
            .unwrap();
        assert!(close(&both[0], &a[0]));
        assert!(close(&both[1], &b[0]));
    }

    struct Recording {
        points: Mutex<Vec<RetrievalPoint>>,
        shift: f32,
    }

    struct Shift(f32);

    impl RetrievalHook for Recording {
        fn name(&self) -> &'static str {
            "recording"
        }

        fn wants(&self, point: RetrievalPoint) -> bool {
            matches!(
                point,
                RetrievalPoint::SequenceStart | RetrievalPoint::LayerEntry { .. }
            )
        }

        fn launch(&self, request: &RetrievalRequest<'_>) -> CoreResult<Box<dyn PendingRetrieval>> {
            self.points.lock().unwrap().push(request.point);
            Ok(Box::new(Shift(self.shift)))
        }
    }

    impl PendingRetrieval for Shift {
        fn join(self: Box<Self>, ctx: &mut ForwardContext<'_>) -> CoreResult<()> {
            if let HiddenState::Host(rows) = &mut ctx.hidden {
                for value in rows.iter_mut() {
                    *value += self.0;
                }
            }
            Ok(())
        }
    }

    #[test]
    fn the_hook_runs_at_the_points_it_wants_and_fuses_into_the_hidden_state() {
        let tokens = [9u32, 8, 7];
        let batch = StepBatch {
            sequences: vec![prefill(&tokens, 0, 0, true)],
        };

        let silent = Arc::new(Recording {
            points: Mutex::new(Vec::new()),
            shift: 0.0,
        });
        let mut driver = Driver::new(tiny_model(Architecture::Qwen2, 3, 32), silent.clone());
        let unchanged = driver.step(&batch, &[progress(&tokens, true)]).unwrap();
        assert_eq!(
            *silent.points.lock().unwrap(),
            vec![
                RetrievalPoint::SequenceStart,
                RetrievalPoint::LayerEntry { layer: 0 },
                RetrievalPoint::LayerEntry { layer: 1 },
            ]
        );

        let mut plain = Driver::new(
            tiny_model(Architecture::Qwen2, 3, 32),
            Arc::new(NoopRetrievalHook),
        );
        let reference = plain.step(&batch, &[progress(&tokens, true)]).unwrap();
        assert!(close(&unchanged[0], &reference[0]));

        let shifting = Arc::new(Recording {
            points: Mutex::new(Vec::new()),
            shift: 1.0,
        });
        let mut driver = Driver::new(tiny_model(Architecture::Qwen2, 3, 32), shifting);
        let shifted = driver.step(&batch, &[progress(&tokens, true)]).unwrap();
        assert!(!close(&shifted[0], &reference[0]));
    }
}
