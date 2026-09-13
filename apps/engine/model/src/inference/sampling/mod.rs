//! Turning logits into tokens: repetition penalty, then greedy selection or temperature, top-k
//! and top-p sampling from a seeded generator.

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
        })
    }

    /// Choose the next token from logits, given the tokens the sequence already holds.
    ///
    /// logits is rewritten in place by the penalty and temperature.
    pub fn sample(&mut self, logits: &mut [f32], history: &[u32]) -> Result<u32, InferenceError> {
        if logits.is_empty() {
            return Err(InferenceError::Runtime("empty logits".to_string()));
        }
        self.apply_repetition_penalty(logits, history);
        if self.temperature == 0.0 {
            return Ok(argmax(logits));
        }

        let mut candidates: Vec<(u32, f32)> = logits
            .iter()
            .enumerate()
            .filter(|(_, logit)| logit.is_finite())
            .map(|(token, &logit)| (token as u32, logit / self.temperature))
            .collect();
        if candidates.is_empty() {
            return Err(InferenceError::Runtime(
                "every logit is non-finite".to_string(),
            ));
        }
        candidates.sort_unstable_by(|a, b| b.1.total_cmp(&a.1));
        if let Some(k) = self.top_k {
            candidates.truncate(k);
        }

        let max = candidates[0].1;
        let mut total = 0.0f64;
        let mut probabilities: Vec<f64> = candidates
            .iter()
            .map(|(_, logit)| {
                let weight = f64::from(logit - max).exp();
                total += weight;
                weight
            })
            .collect();
        for probability in &mut probabilities {
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
            for probability in &mut probabilities {
                *probability /= kept;
            }
        }

        let draw: f64 = self.rng.gen();
        let mut cumulative = 0.0f64;
        for (index, probability) in probabilities.iter().enumerate() {
            cumulative += probability;
            if draw < cumulative {
                return Ok(candidates[index].0);
            }
        }
        Ok(candidates[probabilities.len() - 1].0)
    }

    fn apply_repetition_penalty(&self, logits: &mut [f32], history: &[u32]) {
        if self.repetition_penalty == 1.0 || self.repetition_window == 0 {
            return;
        }
        let start = history.len().saturating_sub(self.repetition_window);
        let mut seen = std::collections::HashSet::new();
        for &token in &history[start..] {
            if !seen.insert(token) {
                continue;
            }
            if let Some(logit) = logits.get_mut(token as usize) {
                if *logit > 0.0 {
                    *logit /= self.repetition_penalty;
                } else {
                    *logit *= self.repetition_penalty;
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

/// Index of the largest logit; the first one on a tie.
fn argmax(logits: &[f32]) -> u32 {
    let mut best = 0;
    for (index, &logit) in logits.iter().enumerate() {
        if logit > logits[best] || logits[best].is_nan() {
            best = index;
        }
    }
    best as u32
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
