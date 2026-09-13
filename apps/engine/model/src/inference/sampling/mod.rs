//! Turning logits into tokens: repetition penalty, then greedy selection or temperature, top-k
//! and top-p sampling from a seeded generator.

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
        validate(config)?;
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

    /// Choose the next token from logits, given the tokens the sequence already holds.
    ///
    /// logits is rewritten in place by the penalty.
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

/// Check sampling settings, naming the field that is out of range.
pub fn validate(config: &SamplingConfig) -> Result<(), InferenceError> {
    let invalid = |message: String| Err(InferenceError::InvalidRequest(message));
    if !(config.temperature >= 0.0 && config.temperature.is_finite()) {
        return invalid(format!(
            "temperature must be >= 0, got {}",
            config.temperature
        ));
    }
    if let Some(top_p) = config.top_p {
        if !(top_p > 0.0 && top_p <= 1.0) {
            return invalid(format!("top_p must be within (0, 1], got {top_p}"));
        }
    }
    if config.top_k == Some(0) {
        return invalid("top_k must be >= 1".to_string());
    }
    if !(config.repetition_penalty > 0.0 && config.repetition_penalty.is_finite()) {
        return invalid(format!(
            "repetition_penalty must be > 0, got {}",
            config.repetition_penalty
        ));
    }
    if config.max_new_tokens == 0 {
        return invalid("max_new_tokens must be >= 1".to_string());
    }
    Ok(())
}

/// Index of the largest logit that is not NaN; the first one on a tie. None when every logit is
/// NaN.
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

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, reason = "assertions in tests")]

    use super::*;

    fn config() -> SamplingConfig {
        SamplingConfig::default()
    }

    #[test]
    fn zero_temperature_picks_the_largest_logit_and_the_first_on_a_tie() {
        let mut sampler = Sampler::new(&config()).unwrap();
        assert_eq!(sampler.sample(&mut [0.1, 3.0, 2.0], &[]).unwrap(), 1);
        assert_eq!(sampler.sample(&mut [5.0, 5.0, 1.0], &[]).unwrap(), 0);
    }

    #[test]
    fn greedy_skips_nan_logits_and_refuses_when_all_are_nan() {
        let mut sampler = Sampler::new(&config()).unwrap();
        assert_eq!(sampler.sample(&mut [1.0, f32::NAN, 2.0], &[]).unwrap(), 2);
        assert_eq!(
            sampler.sample(&mut [f32::NAN, 1.0, f32::NAN], &[]).unwrap(),
            1
        );
        assert!(sampler.sample(&mut [f32::NAN, f32::NAN], &[]).is_err());
    }

    #[test]
    fn a_tiny_temperature_still_picks_the_largest_logit() {
        let settings = SamplingConfig {
            temperature: 1e-40,
            seed: Some(1),
            ..config()
        };
        let mut sampler = Sampler::new(&settings).unwrap();
        for _ in 0..20 {
            assert_eq!(sampler.sample(&mut [-1.0, 2.0, 1.0], &[]).unwrap(), 1);
        }
    }

    #[test]
    fn top_k_keeps_only_the_k_largest() {
        let settings = SamplingConfig {
            temperature: 1.0,
            top_k: Some(2),
            seed: Some(11),
            ..config()
        };
        let mut sampler = Sampler::new(&settings).unwrap();
        for _ in 0..200 {
            let token = sampler.sample(&mut [0.0, 3.0, 1.0, 2.9, 0.5], &[]).unwrap();
            assert!(token == 1 || token == 3, "token {token}");
        }
    }

    #[test]
    fn a_repetition_penalty_moves_greedy_off_a_repeated_token() {
        let settings = SamplingConfig {
            repetition_penalty: 2.0,
            ..config()
        };
        let mut sampler = Sampler::new(&settings).unwrap();
        assert_eq!(sampler.sample(&mut [3.0, 2.0, -1.0], &[0]).unwrap(), 1);
    }

    #[test]
    fn a_penalty_outside_the_window_has_no_effect() {
        let settings = SamplingConfig {
            repetition_penalty: 2.0,
            repetition_window: 1,
            ..config()
        };
        let mut sampler = Sampler::new(&settings).unwrap();
        assert_eq!(sampler.sample(&mut [3.0, 2.0], &[0, 1]).unwrap(), 0);
    }

    #[test]
    fn a_seed_makes_sampling_reproducible() {
        let settings = SamplingConfig {
            temperature: 1.0,
            seed: Some(42),
            ..config()
        };
        let logits: Vec<f32> = (0..50).map(|i| (i as f32 * 0.37).sin()).collect();
        let draw = |settings: &SamplingConfig| {
            let mut sampler = Sampler::new(settings).unwrap();
            (0..20)
                .map(|_| sampler.sample(&mut logits.clone(), &[]).unwrap())
                .collect::<Vec<_>>()
        };
        assert_eq!(draw(&settings), draw(&settings));
    }

    #[test]
    fn top_k_of_one_is_greedy_at_any_temperature() {
        let settings = SamplingConfig {
            temperature: 5.0,
            top_k: Some(1),
            ..config()
        };
        let mut sampler = Sampler::new(&settings).unwrap();
        for _ in 0..20 {
            assert_eq!(sampler.sample(&mut [0.0, 1.0, 0.5], &[]).unwrap(), 1);
        }
    }

    #[test]
    fn top_p_keeps_only_the_nucleus() {
        let settings = SamplingConfig {
            temperature: 1.0,
            top_p: Some(0.5),
            seed: Some(7),
            ..config()
        };
        let mut sampler = Sampler::new(&settings).unwrap();
        for _ in 0..50 {
            let token = sampler.sample(&mut [10.0, 0.0, 0.0, 0.0], &[]).unwrap();
            assert_eq!(token, 0);
        }
    }

    #[test]
    fn sampled_tokens_follow_the_distribution() {
        let settings = SamplingConfig {
            temperature: 1.0,
            seed: Some(3),
            ..config()
        };
        let mut sampler = Sampler::new(&settings).unwrap();
        let mut counts = [0usize; 2];
        let logits = [0.0f32, (3.0f32).ln()];
        for _ in 0..4000 {
            counts[sampler.sample(&mut logits.clone(), &[]).unwrap() as usize] += 1;
        }
        let share = counts[1] as f64 / 4000.0;
        assert!((share - 0.75).abs() < 0.03, "share {share}");
    }

    #[test]
    fn out_of_range_settings_name_the_field() {
        for (settings, field) in [
            (
                SamplingConfig {
                    temperature: -1.0,
                    ..config()
                },
                "temperature",
            ),
            (
                SamplingConfig {
                    top_p: Some(0.0),
                    ..config()
                },
                "top_p",
            ),
            (
                SamplingConfig {
                    top_k: Some(0),
                    ..config()
                },
                "top_k",
            ),
            (
                SamplingConfig {
                    repetition_penalty: 0.0,
                    ..config()
                },
                "repetition_penalty",
            ),
            (
                SamplingConfig {
                    max_new_tokens: 0,
                    ..config()
                },
                "max_new_tokens",
            ),
        ] {
            let error = Sampler::new(&settings).unwrap_err().to_string();
            assert!(error.contains(field), "{error}");
        }
    }
}
