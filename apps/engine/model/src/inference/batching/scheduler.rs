//! Packing sequences into forward steps: decode tokens first, then prefill chunks of sequences
//! already running, then newly admitted prompts, within the batch, token and page limits.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use piramid_core::error::InferenceError;

use crate::inference::architecture::{StepBatch, StepSequence};
use crate::inference::kv_cache::{BlockAllocator, BlockTable, PoolStats};

/// The limits a scheduler packs steps under.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SchedulerLimits {
    /// Sequences running at once.
    pub max_batch_size: usize,
    /// Sequences waiting and running at once.
    pub max_queue_depth: usize,
    /// Tokens computed in one step.
    pub max_batched_tokens: usize,
    /// Whether sequences are admitted into a running batch before it drains.
    pub continuous: bool,
    /// Split prefill across steps.
    pub chunked_prefill: bool,
    /// Tokens per prefill chunk when chunked.
    pub prefill_chunk_tokens: usize,
    /// How long a sequence may wait for admission.
    pub queue_timeout: Option<Duration>,
    /// Longest prompt plus completion.
    pub max_sequence_length: usize,
}

/// One sequence being generated, and the caller state that travels with it.
#[derive(Debug)]
pub struct Sequence<P> {
    /// Identifier unique within the scheduler.
    pub id: u64,
    /// Prompt tokens followed by generated tokens.
    pub tokens: Vec<u32>,
    /// Tokens of the prompt.
    pub prompt_len: usize,
    /// Tokens to generate at most.
    pub max_new_tokens: usize,
    /// When the sequence was submitted.
    pub arrived: Instant,
    /// Prompt tokens first served from shared pages.
    pub cached_prompt_tokens: usize,
    /// Whether any step has computed a token of this sequence.
    pub started: bool,
    /// Tokens whose keys and values are in the cache.
    pub computed: usize,
    table: BlockTable,
    /// Caller state.
    pub payload: P,
}

impl<P> Sequence<P> {
    /// A sequence for a prompt.
    pub fn new(id: u64, prompt: Vec<u32>, max_new_tokens: usize, payload: P) -> Self {
        Self {
            id,
            prompt_len: prompt.len(),
            tokens: prompt,
            max_new_tokens,
            arrived: Instant::now(),
            cached_prompt_tokens: 0,
            started: false,
            computed: 0,
            table: BlockTable::default(),
            payload,
        }
    }

    /// Tokens generated so far.
    pub fn generated(&self) -> usize {
        self.tokens.len() - self.prompt_len
    }

    fn pending(&self) -> usize {
        self.tokens.len() - self.computed
    }
}

/// One sequence's share of a planned step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlannedEntry {
    /// The sequence.
    pub id: u64,
    /// Tokens computed for it in this step.
    pub tokens: usize,
    /// Whether the step computes its first token.
    pub first_step: bool,
    /// Whether the step ends at its last token and so yields logits.
    pub samples: bool,
}

/// A step ready to run, the sequences preempted to make room for it, and the sequences that could
/// not be planned.
#[derive(Debug)]
pub struct PlannedStep<P> {
    /// The batch for the driver.
    pub batch: StepBatch,
    /// One entry per batch sequence, in batch order.
    pub entries: Vec<PlannedEntry>,
    /// Sequences whose pages were released; they return to the front of the queue.
    pub preempted: Vec<u64>,
    /// Sequences removed from the scheduler with their pages released, and why.
    pub failed: Vec<(Sequence<P>, InferenceError)>,
}

impl<P> Default for PlannedStep<P> {
    fn default() -> Self {
        Self {
            batch: StepBatch::default(),
            entries: Vec::new(),
            preempted: Vec::new(),
            failed: Vec::new(),
        }
    }
}

impl<P> PlannedStep<P> {
    /// Whether the step computes nothing.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Queues sequences and plans forward steps over a page pool.
#[derive(Debug)]
pub struct Scheduler<P> {
    limits: SchedulerLimits,
    allocator: BlockAllocator,
    waiting: VecDeque<Sequence<P>>,
    running: Vec<Sequence<P>>,
}

impl<P> Scheduler<P> {
    /// A scheduler over a page pool.
    pub fn new(limits: SchedulerLimits, allocator: BlockAllocator) -> Self {
        Self {
            limits,
            allocator,
            waiting: VecDeque::new(),
            running: Vec::new(),
        }
    }

    /// Queue a sequence, refusing one that can never fit or a full queue.
    pub fn submit(&mut self, sequence: Sequence<P>) -> Result<(), InferenceError> {
        let total = sequence.prompt_len + sequence.max_new_tokens;
        let refuse = |message: String| Err(InferenceError::InvalidRequest(message));
        if sequence.prompt_len == 0 {
            return refuse("the prompt has no tokens".to_string());
        }
        if total > self.limits.max_sequence_length {
            return refuse(format!(
                "{} prompt tokens plus {} new tokens exceed max_sequence_length {}",
                sequence.prompt_len, sequence.max_new_tokens, self.limits.max_sequence_length
            ));
        }
        if total > self.allocator.total_slots() {
            return refuse(format!(
                "{total} tokens exceed the {} token key/value cache",
                self.allocator.total_slots()
            ));
        }
        if !self.limits.chunked_prefill && sequence.prompt_len > self.limits.max_batched_tokens {
            return refuse(format!(
                "{} prompt tokens exceed max_batched_tokens {} without chunked prefill",
                sequence.prompt_len, self.limits.max_batched_tokens
            ));
        }
        if self.waiting.len() + self.running.len() >= self.limits.max_queue_depth {
            return Err(InferenceError::QueueFull(format!(
                "{} sequences queued or running",
                self.limits.max_queue_depth
            )));
        }
        self.waiting.push_back(sequence);
        Ok(())
    }

    /// Remove and return waiting sequences older than the queue timeout.
    pub fn expire(&mut self, now: Instant) -> Vec<Sequence<P>> {
        let Some(timeout) = self.limits.queue_timeout else {
            return Vec::new();
        };
        let expired = |sequence: &Sequence<P>| {
            !sequence.started && now.duration_since(sequence.arrived) > timeout
        };
        if !self.waiting.iter().any(expired) {
            return Vec::new();
        }
        let mut gone = Vec::new();
        let mut kept = VecDeque::with_capacity(self.waiting.len());
        for sequence in self.waiting.drain(..) {
            if expired(&sequence) {
                gone.push(sequence);
            } else {
                kept.push_back(sequence);
            }
        }
        self.waiting = kept;
        gone
    }

    /// Plan the next step.
    pub fn plan(&mut self) -> PlannedStep<P> {
        let mut step = PlannedStep::default();
        let mut budget = self.limits.max_batched_tokens;

        let mut index = 0;
        while index < self.running.len() {
            if self.running[index].pending() != 1 || budget == 0 {
                index += 1;
                continue;
            }
            let needed = self.running[index].tokens.len();
            let Some(at) = self.make_room(index, needed, &mut step) else {
                break;
            };
            budget -= 1;
            index = if self.push_entry(at, 1, &mut step) {
                at + 1
            } else {
                at
            };
        }

        let mut index = 0;
        while index < self.running.len() {
            let sequence = &self.running[index];
            let pending = sequence.pending();
            if pending <= 1 || budget == 0 || step.entries.iter().any(|e| e.id == sequence.id) {
                index += 1;
                continue;
            }
            let Some(count) = self.chunk(pending, budget, step.entries.is_empty()) else {
                index += 1;
                continue;
            };
            let end = sequence.computed + count;
            let Some(at) = self.make_room(index, end, &mut step) else {
                index += 1;
                continue;
            };
            budget -= count;
            index = if self.push_entry(at, count, &mut step) {
                at + 1
            } else {
                at
            };
        }

        let admit = self.limits.continuous || self.running.is_empty();
        while admit && budget > 0 && self.running.len() < self.limits.max_batch_size {
            let Some(mut sequence) = self.waiting.pop_front() else {
                break;
            };
            let (table, cached) = self.allocator.start_sequence(&sequence.tokens);
            if !sequence.started {
                sequence.cached_prompt_tokens = cached;
            }
            sequence.table = table;
            sequence.computed = cached;
            let Some(count) = self.chunk(sequence.pending(), budget, step.entries.is_empty())
            else {
                self.return_to_queue(sequence);
                break;
            };
            let mut table = std::mem::take(&mut sequence.table);
            if self
                .allocator
                .reserve(&mut table, sequence.computed + count)
                .is_err()
            {
                sequence.table = table;
                self.return_to_queue(sequence);
                break;
            }
            sequence.table = table;
            self.running.push(sequence);
            let index = self.running.len() - 1;
            self.push_entry(index, count, &mut step);
            budget -= count;
        }
        step
    }

    fn chunk(&self, pending: usize, budget: usize, alone: bool) -> Option<usize> {
        if self.limits.chunked_prefill {
            Some(pending.min(self.limits.prefill_chunk_tokens).min(budget))
        } else if pending <= budget || alone {
            Some(pending)
        } else {
            None
        }
    }

    fn return_to_queue(&mut self, mut sequence: Sequence<P>) {
        let table = std::mem::take(&mut sequence.table);
        self.allocator.release(table);
        sequence.computed = 0;
        self.waiting.push_front(sequence);
    }

    /// Reserve pages for the sequence at index to hold needed tokens, preempting the most recently
    /// arrived running sequences outside the step until it fits. Returns the sequence's index after
    /// preemption, or None when preempting every other sequence cannot make room.
    fn make_room(
        &mut self,
        mut index: usize,
        needed: usize,
        step: &mut PlannedStep<P>,
    ) -> Option<usize> {
        loop {
            let mut table = std::mem::take(&mut self.running[index].table);
            let reserved = self.allocator.reserve(&mut table, needed).is_ok();
            self.running[index].table = table;
            if reserved {
                return Some(index);
            }
            let victim = (0..self.running.len()).rev().find(|&candidate| {
                candidate != index
                    && !step
                        .entries
                        .iter()
                        .any(|e| e.id == self.running[candidate].id)
            })?;
            let sequence = self.running.remove(victim);
            if victim < index {
                index -= 1;
            }
            step.preempted.push(sequence.id);
            self.return_to_queue(sequence);
        }
    }

    /// Add count tokens of the sequence at index to the step. When its pages cannot take them, the
    /// sequence is removed from running with its pages released and moved to the step's failed
    /// list. Returns whether the entry was added.
    fn push_entry(&mut self, index: usize, count: usize, step: &mut PlannedStep<P>) -> bool {
        let sequence = &mut self.running[index];
        let start = sequence.computed;
        let end = start + count;
        let write_slots = match self.allocator.advance(&mut sequence.table, count) {
            Ok(write_slots) => write_slots,
            Err(error) => {
                let mut sequence = self.running.remove(index);
                self.allocator.release(std::mem::take(&mut sequence.table));
                step.failed.push((sequence, error));
                return false;
            }
        };
        let context_slots = self.allocator.slots(&sequence.table, 0, end);
        let first_step = !sequence.started;
        sequence.started = true;
        sequence.computed = end;
        let samples = end == sequence.tokens.len();
        step.batch.sequences.push(StepSequence {
            tokens: sequence.tokens[start..end].to_vec(),
            start,
            write_slots,
            context_slots,
            logits: samples,
        });
        step.entries.push(PlannedEntry {
            id: sequence.id,
            tokens: count,
            first_step,
            samples,
        });
        true
    }

    /// The running sequence with an id.
    pub fn running(&self, id: u64) -> Option<&Sequence<P>> {
        self.running.iter().find(|sequence| sequence.id == id)
    }

    /// The running sequence with an id, mutably.
    pub fn running_mut(&mut self, id: u64) -> Option<&mut Sequence<P>> {
        self.running.iter_mut().find(|sequence| sequence.id == id)
    }

    /// Append a sampled token to a running sequence.
    pub fn append(&mut self, id: u64, token: u32) {
        if let Some(sequence) = self.running_mut(id) {
            sequence.tokens.push(token);
        }
    }

    /// Remove a running or waiting sequence, releasing its pages. Its computed prefix becomes
    /// shareable with later prompts.
    pub fn remove(&mut self, id: u64) -> Option<Sequence<P>> {
        let mut sequence = if let Some(index) = self.running.iter().position(|s| s.id == id) {
            self.running.remove(index)
        } else {
            let index = self.waiting.iter().position(|s| s.id == id)?;
            self.waiting.remove(index)?
        };
        let table = std::mem::take(&mut sequence.table);
        let computed = sequence.computed.min(sequence.tokens.len());
        self.allocator
            .publish_prefix(&table, &sequence.tokens[..computed]);
        self.allocator.release(table);
        Some(sequence)
    }

    /// Remove every sequence, waiting and running.
    pub fn drain(&mut self) -> Vec<Sequence<P>> {
        self.remove_where(|_| true)
    }

    /// Remove every running or waiting sequence the predicate selects, releasing its pages.
    pub fn remove_where(
        &mut self,
        mut predicate: impl FnMut(&Sequence<P>) -> bool,
    ) -> Vec<Sequence<P>> {
        let ids: Vec<u64> = self
            .running
            .iter()
            .chain(self.waiting.iter())
            .filter(|sequence| predicate(sequence))
            .map(|sequence| sequence.id)
            .collect();
        ids.into_iter().filter_map(|id| self.remove(id)).collect()
    }

    /// Ids of every sequence, running first.
    pub fn ids(&self) -> Vec<u64> {
        self.running
            .iter()
            .chain(self.waiting.iter())
            .map(|sequence| sequence.id)
            .collect()
    }

    /// Sequences waiting for admission.
    pub fn waiting(&self) -> usize {
        self.waiting.len()
    }

    /// Sequences running.
    pub fn running_count(&self) -> usize {
        self.running.len()
    }

    /// Whether nothing is queued or running.
    pub fn is_idle(&self) -> bool {
        self.waiting.is_empty() && self.running.is_empty()
    }

    /// Page pool counters.
    pub fn pool(&self) -> PoolStats {
        self.allocator.stats()
    }
}
