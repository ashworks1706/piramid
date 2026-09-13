//! Scores of one generation: answer match, retrieval recall, decode rate, and latency
//! percentiles across generations.

use std::time::Duration;

/// Lowercase, drop punctuation and the articles a, an and the, and collapse whitespace.
pub fn normalize_answer(text: &str) -> String {
    let lowered: String = text
        .to_lowercase()
        .chars()
        .filter(|c| !c.is_ascii_punctuation())
        .collect();
    lowered
        .split_whitespace()
        .filter(|word| !matches!(*word, "a" | "an" | "the"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Whether any normalized answer appears as a whole word sequence in the normalized output.
pub fn exact_match(output: &str, answers: &[String]) -> bool {
    let output = format!(" {} ", normalize_answer(output));
    answers.iter().any(|answer| {
        let answer = normalize_answer(answer);
        !answer.is_empty() && output.contains(&format!(" {answer} "))
    })
}

/// Whether any gold id is among the retrieved ids. None when there is no gold id.
pub fn recall_at_k(retrieved: &[String], gold: &[&str]) -> Option<bool> {
    if gold.is_empty() {
        return None;
    }
    Some(
        retrieved
            .iter()
            .any(|id| gold.iter().any(|gold| gold == id)),
    )
}

/// Tokens per second after the first token. None with fewer than two tokens or no decode time.
pub fn decode_tokens_per_second(
    completion_tokens: usize,
    time_to_first_token: Option<Duration>,
    total: Duration,
) -> Option<f64> {
    let first = time_to_first_token?;
    let decoded = completion_tokens.checked_sub(1).filter(|n| *n > 0)?;
    let seconds = total.checked_sub(first)?.as_secs_f64();
    (seconds > 0.0).then(|| decoded as f64 / seconds)
}

/// The q quantile of values, from 0 to 1, interpolated linearly between ranks. None when empty.
pub fn percentile(values: &[f64], q: f64) -> Option<f64> {
    if values.is_empty() || !(0.0..=1.0).contains(&q) {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let rank = q * (sorted.len() - 1) as f64;
    let lower = rank.floor() as usize;
    let upper = rank.ceil() as usize;
    let fraction = rank - lower as f64;
    Some(sorted[lower] + (sorted[upper] - sorted[lower]) * fraction)
}

/// Arithmetic mean. None when empty.
pub fn mean(values: &[f64]) -> Option<f64> {
    (!values.is_empty()).then(|| values.iter().sum::<f64>() / values.len() as f64)
}
