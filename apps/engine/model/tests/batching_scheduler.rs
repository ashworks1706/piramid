//! Packing sequences into forward steps under batch, token and page limits.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "assertions in tests"
)]

use std::time::{Duration, Instant};

use piramid_core::error::InferenceError;
use piramid_model::inference::batching::{PlannedStep, Scheduler, SchedulerLimits, Sequence};
use piramid_model::inference::kv_cache::BlockAllocator;

fn limits() -> SchedulerLimits {
    SchedulerLimits {
        max_batch_size: 4,
        max_queue_depth: 8,
        max_batched_tokens: 64,
        continuous: true,
        chunked_prefill: false,
        prefill_chunk_tokens: 16,
        queue_timeout: None,
        max_sequence_length: 256,
    }
}

fn scheduler(limits: SchedulerLimits, blocks: usize) -> Scheduler<()> {
    Scheduler::new(limits, BlockAllocator::new(blocks, 4, false))
}

fn prompt(len: usize) -> Vec<u32> {
    (0..len as u32).collect()
}

/// Plan a step and append a token to every sampling entry, as a worker would.
fn run(scheduler: &mut Scheduler<()>) -> PlannedStep<()> {
    let step = scheduler.plan();
    for entry in &step.entries {
        if entry.samples {
            scheduler.append(entry.id, 1);
        }
    }
    step
}

#[test]
fn a_prompt_prefills_then_decodes_one_token_per_step() {
    let mut scheduler = scheduler(limits(), 16);
    scheduler
        .submit(Sequence::new(1, prompt(6), 4, ()))
        .unwrap();
    let step = run(&mut scheduler);
    assert_eq!(step.batch.sequences[0].tokens.len(), 6);
    assert!(step.entries[0].first_step && step.entries[0].samples);

    let step = run(&mut scheduler);
    let sequence = &step.batch.sequences[0];
    assert_eq!((sequence.start, sequence.tokens.len()), (6, 1));
    assert_eq!(sequence.context_slots.len(), 7);
    assert!(!step.entries[0].first_step);
}

#[test]
fn chunked_prefill_splits_a_long_prompt_and_samples_only_at_its_end() {
    let settings = SchedulerLimits {
        chunked_prefill: true,
        prefill_chunk_tokens: 4,
        ..limits()
    };
    let mut scheduler = scheduler(settings, 16);
    scheduler
        .submit(Sequence::new(1, prompt(10), 2, ()))
        .unwrap();
    let sizes: Vec<(usize, bool)> = (0..3)
        .map(|_| {
            let step = run(&mut scheduler);
            (step.entries[0].tokens, step.entries[0].samples)
        })
        .collect();
    assert_eq!(sizes, vec![(4, false), (4, false), (2, true)]);
}

#[test]
fn decodes_are_scheduled_before_new_prompts_within_the_token_budget() {
    let settings = SchedulerLimits {
        max_batched_tokens: 8,
        ..limits()
    };
    let mut scheduler = scheduler(settings, 32);
    scheduler
        .submit(Sequence::new(1, prompt(7), 4, ()))
        .unwrap();
    run(&mut scheduler);
    scheduler
        .submit(Sequence::new(2, prompt(7), 4, ()))
        .unwrap();
    scheduler
        .submit(Sequence::new(3, prompt(7), 4, ()))
        .unwrap();
    let step = run(&mut scheduler);
    let ids: Vec<u64> = step.entries.iter().map(|e| e.id).collect();
    assert_eq!(
        ids,
        vec![1, 2],
        "one decode and one prompt fit eight tokens"
    );
    assert_eq!(scheduler.waiting(), 1);
}

#[test]
fn without_continuous_batching_new_prompts_wait_for_the_batch_to_drain() {
    let settings = SchedulerLimits {
        continuous: false,
        ..limits()
    };
    let mut scheduler = scheduler(settings, 32);
    scheduler
        .submit(Sequence::new(1, prompt(3), 4, ()))
        .unwrap();
    run(&mut scheduler);
    scheduler
        .submit(Sequence::new(2, prompt(3), 4, ()))
        .unwrap();
    let step = run(&mut scheduler);
    assert_eq!(step.entries.len(), 1);
    scheduler.remove(1);
    let step = run(&mut scheduler);
    assert_eq!(step.entries[0].id, 2);
}

#[test]
fn running_out_of_pages_preempts_the_newest_sequence_for_recompute() {
    let mut scheduler = scheduler(limits(), 4);
    scheduler
        .submit(Sequence::new(1, prompt(8), 8, ()))
        .unwrap();
    scheduler
        .submit(Sequence::new(2, prompt(8), 8, ()))
        .unwrap();
    run(&mut scheduler);
    assert_eq!(scheduler.running_count(), 2);
    let step = run(&mut scheduler);
    assert_eq!(step.preempted, vec![2]);
    assert_eq!(
        step.entries.iter().map(|e| e.id).collect::<Vec<_>>(),
        vec![1]
    );
    assert_eq!(scheduler.waiting(), 1);

    scheduler.remove(1);
    let step = run(&mut scheduler);
    assert_eq!(step.entries[0].id, 2);
    assert_eq!(
        step.entries[0].tokens, 9,
        "the prompt and its generated token are recomputed"
    );
    assert!(!step.entries[0].first_step);
}

#[test]
fn a_decode_that_preempts_an_earlier_sequence_still_runs_in_the_same_step() {
    let settings = SchedulerLimits {
        chunked_prefill: true,
        prefill_chunk_tokens: 4,
        max_batched_tokens: 8,
        ..limits()
    };
    let mut scheduler = scheduler(settings, 2);
    scheduler
        .submit(Sequence::new(1, prompt(6), 1, ()))
        .unwrap();
    scheduler
        .submit(Sequence::new(2, prompt(4), 2, ()))
        .unwrap();
    run(&mut scheduler);
    let step = run(&mut scheduler);
    assert_eq!(step.preempted, vec![1]);
    assert_eq!(
        step.entries
            .iter()
            .map(|e| (e.id, e.tokens))
            .collect::<Vec<_>>(),
        vec![(2, 1)]
    );
}

#[test]
fn prefills_that_exhaust_the_pool_preempt_instead_of_stalling() {
    let settings = SchedulerLimits {
        chunked_prefill: true,
        prefill_chunk_tokens: 5,
        ..limits()
    };
    let mut scheduler = scheduler(settings, 12);
    for id in 1..=3 {
        scheduler
            .submit(Sequence::new(id, prompt(25), 12, ()))
            .unwrap();
    }
    let mut finished = 0;
    for _ in 0..500 {
        if scheduler.is_idle() {
            break;
        }
        let step = run(&mut scheduler);
        assert!(
            !step.is_empty(),
            "a step with sequences queued computes nothing"
        );
        for entry in &step.entries {
            let done = scheduler
                .running(entry.id)
                .is_some_and(|sequence| sequence.generated() >= sequence.max_new_tokens);
            if done {
                scheduler.remove(entry.id);
                finished += 1;
            }
        }
    }
    assert_eq!(finished, 3);
}

#[test]
fn a_sequence_whose_pages_cannot_take_its_tokens_is_failed_with_the_allocator_error() {
    let mut scheduler = scheduler(limits(), 16);
    scheduler
        .submit(Sequence::new(1, prompt(4), 4, ()))
        .unwrap();
    scheduler
        .submit(Sequence::new(2, prompt(3), 4, ()))
        .unwrap();
    run(&mut scheduler);
    scheduler.running_mut(1).unwrap().computed = 0;

    let step = scheduler.plan();
    assert_eq!(step.failed.len(), 1);
    let (sequence, error) = &step.failed[0];
    assert_eq!(sequence.id, 1);
    assert!(matches!(error, InferenceError::Runtime(_)), "{error}");
    assert_eq!(
        step.entries.iter().map(|e| e.id).collect::<Vec<_>>(),
        vec![2]
    );
    assert_eq!(scheduler.ids(), vec![2]);
}

#[test]
fn remove_where_takes_matching_sequences_from_both_queues() {
    let settings = SchedulerLimits {
        max_batch_size: 1,
        ..limits()
    };
    let mut scheduler = scheduler(settings, 16);
    for id in 1..=3 {
        scheduler
            .submit(Sequence::new(id, prompt(3), 2, ()))
            .unwrap();
    }
    run(&mut scheduler);
    let removed: Vec<u64> = scheduler
        .remove_where(|sequence| sequence.id != 2)
        .iter()
        .map(|sequence| sequence.id)
        .collect();
    assert_eq!(removed, vec![1, 3]);
    assert_eq!(scheduler.ids(), vec![2]);
    assert_eq!(scheduler.pool().used_blocks, 0);
}

#[test]
fn requests_that_can_never_fit_are_refused_at_submit() {
    let mut scheduler = scheduler(limits(), 4);
    let error = scheduler
        .submit(Sequence::new(1, prompt(10), 10, ()))
        .unwrap_err();
    assert!(error.to_string().contains("cache"), "{error}");
    let error = scheduler
        .submit(Sequence::new(2, prompt(200), 100, ()))
        .unwrap_err();
    assert!(error.to_string().contains("max_sequence_length"), "{error}");
    let error = scheduler
        .submit(Sequence::new(3, Vec::new(), 1, ()))
        .unwrap_err();
    assert!(error.to_string().contains("no tokens"), "{error}");
}

#[test]
fn a_full_queue_is_refused_and_old_waiters_expire() {
    let settings = SchedulerLimits {
        max_queue_depth: 1,
        queue_timeout: Some(Duration::from_millis(0)),
        ..limits()
    };
    let mut scheduler = scheduler(settings, 16);
    scheduler
        .submit(Sequence::new(1, prompt(2), 2, ()))
        .unwrap();
    let error = scheduler
        .submit(Sequence::new(2, prompt(2), 2, ()))
        .unwrap_err();
    assert!(matches!(error, InferenceError::QueueFull(_)));
    std::thread::sleep(Duration::from_millis(2));
    assert_eq!(scheduler.expire(Instant::now()).len(), 1);
    assert!(scheduler.is_idle());
}
