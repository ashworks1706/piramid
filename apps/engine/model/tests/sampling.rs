//! Sampling a token from logits under the sampling settings.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "assertions in tests"
)]

use piramid_core::config::SamplingConfig;
use piramid_core::error::InferenceError;
use piramid_model::inference::sampling::Sampler;

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
fn greedy_refuses_the_settings_only_sampling_reads() {
    for (settings, field) in [
        (
            SamplingConfig {
                top_p: Some(0.9),
                ..config()
            },
            "top_p",
        ),
        (
            SamplingConfig {
                top_k: Some(4),
                ..config()
            },
            "top_k",
        ),
        (
            SamplingConfig {
                seed: Some(1),
                ..config()
            },
            "seed",
        ),
    ] {
        let error = Sampler::new(&settings).unwrap_err();
        assert!(
            matches!(error, InferenceError::InvalidRequest(_)),
            "{error}"
        );
        assert!(error.to_string().contains(field), "{error}");
        assert!(error.to_string().contains("temperature"), "{error}");
    }
    let sampling = SamplingConfig {
        temperature: 0.5,
        top_p: Some(0.9),
        top_k: Some(4),
        seed: Some(1),
        ..config()
    };
    assert!(Sampler::new(&sampling).is_ok());
}

#[test]
fn an_empty_stop_string_is_refused() {
    let settings = SamplingConfig {
        stop: vec!["end".to_string(), String::new()],
        ..config()
    };
    let error = Sampler::new(&settings).unwrap_err();
    assert!(
        error.to_string().contains("stop strings must not be empty"),
        "{error}"
    );
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
