//! The retrieval hook contract and the no-op hook.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "assertions in tests"
)]

use piramid_core::error::Result;
use piramid_model::fusion::{
    ForwardContext, HiddenState, NoopRetrievalHook, PendingRetrieval, RetrievalHook,
    RetrievalPoint, RetrievalRequest,
};

/// A hook that records the point it launched at and adds one to the host hidden state on join.
struct AddOne;

struct AddOnePending {
    launched_at: RetrievalPoint,
}

impl RetrievalHook for AddOne {
    fn name(&self) -> &'static str {
        "add-one"
    }

    fn wants(&self, point: RetrievalPoint) -> bool {
        matches!(point, RetrievalPoint::SequenceStart)
    }

    fn launch(&self, request: &RetrievalRequest<'_>) -> Result<Box<dyn PendingRetrieval>> {
        Ok(Box::new(AddOnePending {
            launched_at: request.point,
        }))
    }
}

impl PendingRetrieval for AddOnePending {
    fn join(self: Box<Self>, ctx: &mut ForwardContext<'_>) -> Result<()> {
        assert_eq!(self.launched_at, ctx.point);
        match &mut ctx.hidden {
            HiddenState::Host(state) => {
                for slot in state.iter_mut() {
                    *slot += 1.0;
                }
                Ok(())
            }
            HiddenState::Device(_) => Ok(()),
        }
    }
}

#[test]
fn a_hook_launches_then_fuses_on_join() {
    let hook = AddOne;
    let tokens = [1u32, 2, 3];
    let mut hidden = vec![0.0f32; 4];

    let request = RetrievalRequest {
        point: RetrievalPoint::SequenceStart,
        tokens: &tokens,
        hidden_dim: hidden.len(),
        stream: None,
    };
    let pending = hook.launch(&request).unwrap();

    let mut ctx = ForwardContext {
        point: RetrievalPoint::SequenceStart,
        hidden_dim: hidden.len(),
        hidden: HiddenState::Host(&mut hidden),
        stream: None,
    };
    pending.join(&mut ctx).unwrap();

    assert_eq!(hidden, vec![1.0; 4]);
}

#[test]
fn wants_gates_the_points_a_hook_runs_at() {
    let hook = AddOne;
    assert!(hook.wants(RetrievalPoint::SequenceStart));
    assert!(!hook.wants(RetrievalPoint::LayerEntry { layer: 0 }));
    assert!(!hook.wants(RetrievalPoint::ChunkBoundary { chunk: 0 }));
}

#[test]
fn the_noop_hook_leaves_the_pass_untouched() {
    let hook = NoopRetrievalHook;
    let tokens = [7u32];
    let mut hidden = vec![0.5f32; 3];

    assert!(!hook.wants(RetrievalPoint::SequenceStart));
    let pending = hook
        .launch(&RetrievalRequest {
            point: RetrievalPoint::SequenceStart,
            tokens: &tokens,
            hidden_dim: hidden.len(),
            stream: None,
        })
        .unwrap();

    let mut ctx = ForwardContext {
        point: RetrievalPoint::SequenceStart,
        hidden_dim: hidden.len(),
        hidden: HiddenState::Host(&mut hidden),
        stream: None,
    };
    pending.join(&mut ctx).unwrap();

    assert_eq!(hidden, vec![0.5; 3]);
}
