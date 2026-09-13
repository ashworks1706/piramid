//! Turns logits into tokens: repetition penalty, then greedy or temperature/top-k/top-p sampling.

use std::collections::HashSet;

use piramid_core::config::SamplingConfig;
use piramid_core::error::InferenceError;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

/// Draws tokens for one sequence under one sampling configuration.
#[derive(Debug)]
pub struct Sampler {
    temperature: f32,
    top_p: Option<f32>,
    top_k: Option<usize>,
    repetition_penalty: f32,
    repetition_window: usize,
    rng: StdRng,
    seen: HashSet<u32>,
    candidates: Vec<(u32, f64)>,
    probabilities: Vec<f64>,
}

impl Sampler {
    /// A sampler for the given settings. A seed makes the draws reproducible.
    pub fn new(config: &SamplingConfig) -> Result<Self, InferenceError> {
        config.validate().map_err(InferenceError::InvalidRequest)?;
        let rng = match config.seed {
            Some(seed) => StdRng::seed_from_u64(seed),
            None => StdRng::from_entropy(),
        };
        Ok(Self {
            temperature: config.temperature,
            top_p: config.top_p,
            top_k: config.top_k,
            repetition_penalty: config.repetition_penalty,
            repetition_window: config.repetition_window,
            rng,
            seen: HashSet::new(),
            candidates: Vec::new(),
            probabilities: Vec::new(),
        })
    }

    /// Choose the next token from logits, rewritten in place by the penalty, given the history.
    pub fn sample(&mut self, logits: &mut [f32], history: &[u32]) -> Result<u32, InferenceError> {
        if logits.is_empty() {
            return Err(InferenceError::Runtime("empty logits".to_string()));
        }
        if u32::try_from(logits.len()).is_err() {
            return Err(InferenceError::Runtime(format!(
                "{} logits exceed the u32 token id range",
                logits.len()
            )));
        }
        self.apply_repetition_penalty(logits, history);
        if self.temperature == 0.0 {
            return argmax(logits)
                .ok_or_else(|| InferenceError::Runtime("every logit is NaN".to_string()));
        }

        let temperature = f64::from(self.temperature);
        let candidates = &mut self.candidates;
        candidates.clear();
        candidates.extend(
            logits
                .iter()
                .enumerate()
                .filter(|(_, logit)| logit.is_finite())
                .map(|(token, &logit)| (token_id(token), f64::from(logit) / temperature)),
        );
        if candidates.is_empty() {
            return Err(InferenceError::Runtime(
                "every logit is non-finite".to_string(),
            ));
        }
        let descending = |a: &(u32, f64), b: &(u32, f64)| b.1.total_cmp(&a.1);
        if let Some(k) = self.top_k.filter(|&k| k < candidates.len()) {
            candidates.select_nth_unstable_by(k - 1, descending);
            candidates.truncate(k);
        }
        candidates.sort_unstable_by(descending);

        let max = candidates[0].1;
        let probabilities = &mut self.probabilities;
        probabilities.clear();
        probabilities.extend(candidates.iter().map(|(_, logit)| (logit - max).exp()));
        let total: f64 = probabilities.iter().sum();
        for probability in probabilities.iter_mut() {
            *probability /= total;
        }

        if let Some(top_p) = self.top_p {
            let mut cumulative = 0.0f64;
            let mut keep = probabilities.len();
            for (index, probability) in probabilities.iter().enumerate() {
                cumulative += probability;
                if cumulative >= f64::from(top_p) {
                    keep = index + 1;
                    break;
                }
            }
            probabilities.truncate(keep);
            let kept: f64 = probabilities.iter().sum();
            for probability in probabilities.iter_mut() {
                *probability /= kept;
            }
        }

        let draw: f64 = self.rng.gen();
        let mut cumulative = 0.0f64;
        for (&(token, _), probability) in candidates.iter().zip(probabilities.iter()) {
            cumulative += probability;
            if draw < cumulative {
                return Ok(token);
            }
        }
        Ok(candidates[probabilities.len() - 1].0)
    }

    fn apply_repetition_penalty(&mut self, logits: &mut [f32], history: &[u32]) {
        if self.repetition_penalty == 1.0 || self.repetition_window == 0 {
            return;
        }
        let penalty = self.repetition_penalty;
        let start = history.len().saturating_sub(self.repetition_window);
        self.seen.clear();
        for &token in &history[start..] {
            if !self.seen.insert(token) {
                continue;
            }
            if let Some(logit) = logits.get_mut(token as usize) {
                if *logit > 0.0 {
                    *logit /= penalty;
                } else {
                    *logit *= penalty;
                }
            }
        }
    }
}

/// Index of the largest logit that is not NaN, first on a tie; None when every logit is NaN.
fn argmax(logits: &[f32]) -> Option<u32> {
    let mut best: Option<(usize, f32)> = None;
    for (index, &logit) in logits.iter().enumerate() {
        if logit.is_nan() {
            continue;
        }
        if best.is_none_or(|(_, top)| logit > top) {
            best = Some((index, logit));
        }
    }
    best.map(|(index, _)| token_id(index))
}

/// A logit index as a token id. Callers check the logits fit the u32 range.
fn token_id(index: usize) -> u32 {
    u32::try_from(index).unwrap_or(u32::MAX)
}
