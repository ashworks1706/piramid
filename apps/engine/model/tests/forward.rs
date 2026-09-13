//! The forward-pass driver over a tiny model, with and without a retrieval hook.
#![cfg(feature = "inference-candle")]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "assertions in tests"
)]

use std::sync::{Arc, Mutex};

use piramid_core::config::DeviceSelection;
use piramid_core::error::Result as CoreResult;
use piramid_model::fusion::{
    ForwardContext, HiddenState, NoopRetrievalHook, PendingRetrieval, RetrievalHook,
    RetrievalPoint, RetrievalRequest,
};
use piramid_model::inference::architecture::{Architecture, StepBatch, StepSequence};
use piramid_model::inference::backends::candle::qwen::testing::{tiny_model, tiny_model_on};
use piramid_model::inference::backends::candle::runtime::CandleRuntime;
use piramid_model::inference::forward::{Driver, SequenceProgress};

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

/// Stream ids seen at launch and at join, per hook call.
type SeenStreams = Arc<Mutex<Vec<(Option<u64>, Option<u64>)>>>;

struct StreamIds {
    seen: SeenStreams,
}

struct StreamIdsPending {
    launched: Option<u64>,
    seen: SeenStreams,
}

impl RetrievalHook for StreamIds {
    fn name(&self) -> &'static str {
        "stream-ids"
    }

    fn wants(&self, point: RetrievalPoint) -> bool {
        point == RetrievalPoint::SequenceStart
    }

    fn launch(&self, request: &RetrievalRequest<'_>) -> CoreResult<Box<dyn PendingRetrieval>> {
        Ok(Box::new(StreamIdsPending {
            launched: request.stream.map(|stream| stream.id()),
            seen: self.seen.clone(),
        }))
    }
}

impl PendingRetrieval for StreamIdsPending {
    fn join(self: Box<Self>, ctx: &mut ForwardContext<'_>) -> CoreResult<()> {
        self.seen
            .lock()
            .unwrap()
            .push((self.launched, ctx.stream.map(|stream| stream.id())));
        Ok(())
    }
}

fn stream_ids_on(selection: DeviceSelection) -> Vec<(Option<u64>, Option<u64>)> {
    let runtime = CandleRuntime::open(selection).unwrap();
    let seen = Arc::new(Mutex::new(Vec::new()));
    let hook = Arc::new(StreamIds { seen: seen.clone() });
    let tokens = [4u32, 5, 6];
    let batch = StepBatch {
        sequences: vec![prefill(&tokens, 0, 0, true)],
    };
    let mut driver = Driver::new(
        tiny_model_on(Architecture::Qwen3, 4, 16, runtime.device()),
        hook,
    );
    driver.step(&batch, &[progress(&tokens, true)]).unwrap();
    let ids = seen.lock().unwrap().clone();
    ids
}

#[test]
fn launch_on_the_cpu_is_told_there_is_no_model_stream() {
    assert_eq!(stream_ids_on(DeviceSelection::Cpu), vec![(None, None)]);
}

#[cfg(feature = "gpu-cuda")]
#[test]
#[ignore = "needs a CUDA device"]
fn launch_on_a_device_is_given_the_stream_join_orders_against() {
    let ids = stream_ids_on(DeviceSelection::Cuda(0));
    assert_eq!(ids.len(), 1);
    assert!(ids[0].0.is_some());
    assert_eq!(ids[0].0, ids[0].1);
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
