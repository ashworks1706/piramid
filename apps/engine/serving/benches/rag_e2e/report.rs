//! Measurements of one question under one arm, their per-arm summary, and the markdown table
//! rendered from it.

use serde::Serialize;

use super::scoring::{mean, percentile};

/// What one question measured under one arm.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Record {
    /// Question id.
    pub question: String,
    /// Arm name.
    pub arm: &'static str,
    /// Embedding the question, in milliseconds. None when no retrieval ran in process.
    pub embed_ms: Option<f64>,
    /// Searching the collection, in milliseconds. For the HTTP arm, the search time the server
    /// reports, in whole milliseconds.
    pub search_ms: Option<f64>,
    /// Reading passage ids and texts out of the hits, in milliseconds.
    pub fetch_ms: Option<f64>,
    /// From the question to passages in hand, in milliseconds. For the HTTP arm, the round trip.
    pub retrieval_ms: Option<f64>,
    /// From admission by the engine to the first token, in milliseconds.
    pub prefill_ms: Option<f64>,
    /// From the question to the first token, in milliseconds, retrieval and prompt building
    /// included.
    pub ttft_ms: Option<f64>,
    /// Tokens per second after the first token.
    pub decode_tokens_per_sec: Option<f64>,
    /// Tokens in the prompt.
    pub prompt_tokens: usize,
    /// Tokens generated.
    pub completion_tokens: usize,
    /// Whether a gold passage was retrieved. None when no retrieval ran or the question has no
    /// gold passage.
    pub recall: Option<bool>,
    /// Whether a normalized answer appears in the normalized output.
    pub exact_match: bool,
    /// The generated text.
    pub output: String,
}

/// Median and 95th percentile of one latency.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Spread {
    /// 50th percentile.
    pub p50: f64,
    /// 95th percentile.
    pub p95: f64,
}

/// Every record of one arm reduced to its percentiles and rates.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ArmSummary {
    /// Arm name.
    pub arm: &'static str,
    /// Questions recorded.
    pub questions: usize,
    /// Question embedding latency.
    pub embed_ms: Option<Spread>,
    /// Search latency.
    pub search_ms: Option<Spread>,
    /// Passage read latency.
    pub fetch_ms: Option<Spread>,
    /// Question to passages latency.
    pub retrieval_ms: Option<Spread>,
    /// Engine admission to first token latency.
    pub prefill_ms: Option<Spread>,
    /// Question to first token latency.
    pub ttft_ms: Option<Spread>,
    /// Mean decode rate over records that have one.
    pub decode_tokens_per_sec: Option<f64>,
    /// Mean tokens generated.
    pub completion_tokens: Option<f64>,
    /// Share of records with a gold passage retrieved, over records that report recall.
    pub recall_at_k: Option<f64>,
    /// Share of records whose output matched an answer.
    pub exact_match: Option<f64>,
}

/// Summarize the records of arm.
pub fn summarize(arm: &'static str, records: &[Record]) -> ArmSummary {
    let mine: Vec<&Record> = records.iter().filter(|record| record.arm == arm).collect();
    let spread = |field: fn(&Record) -> Option<f64>| {
        let values: Vec<f64> = mine.iter().filter_map(|record| field(record)).collect();
        Some(Spread {
            p50: percentile(&values, 0.5)?,
            p95: percentile(&values, 0.95)?,
        })
    };
    let share = |hits: usize, total: usize| (total > 0).then(|| hits as f64 / total as f64);
    let recalls: Vec<bool> = mine.iter().filter_map(|record| record.recall).collect();
    let decode: Vec<f64> = mine
        .iter()
        .filter_map(|record| record.decode_tokens_per_sec)
        .collect();
    let completion: Vec<f64> = mine
        .iter()
        .map(|record| record.completion_tokens as f64)
        .collect();
    ArmSummary {
        arm,
        questions: mine.len(),
        embed_ms: spread(|record| record.embed_ms),
        search_ms: spread(|record| record.search_ms),
        fetch_ms: spread(|record| record.fetch_ms),
        retrieval_ms: spread(|record| record.retrieval_ms),
        prefill_ms: spread(|record| record.prefill_ms),
        ttft_ms: spread(|record| record.ttft_ms),
        decode_tokens_per_sec: mean(&decode),
        completion_tokens: mean(&completion),
        recall_at_k: share(recalls.iter().filter(|hit| **hit).count(), recalls.len()),
        exact_match: share(
            mine.iter().filter(|record| record.exact_match).count(),
            mine.len(),
        ),
    }
}

/// A markdown table with one row per arm. k labels the recall column.
pub fn markdown_table(summaries: &[ArmSummary], k: usize) -> String {
    let spread = |value: Option<Spread>| {
        value.map_or_else(
            || "-".to_string(),
            |spread| format!("{:.1} / {:.1}", spread.p50, spread.p95),
        )
    };
    let number = |value: Option<f64>| value.map_or_else(|| "-".to_string(), |v| format!("{v:.1}"));
    let percent = |value: Option<f64>| {
        value.map_or_else(|| "-".to_string(), |v| format!("{:.1}%", v * 100.0))
    };

    let mut table = format!(
        "| arm | questions | embed ms p50 / p95 | search ms p50 / p95 | fetch ms p50 / p95 \
         | retrieval ms p50 / p95 | prefill ms p50 / p95 | TTFT ms p50 / p95 | decode tok/s \
         | recall@{k} | EM |\n"
    );
    table.push_str("|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|\n");
    for summary in summaries {
        table.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |\n",
            summary.arm,
            summary.questions,
            spread(summary.embed_ms),
            spread(summary.search_ms),
            spread(summary.fetch_ms),
            spread(summary.retrieval_ms),
            spread(summary.prefill_ms),
            spread(summary.ttft_ms),
            number(summary.decode_tokens_per_sec),
            percent(summary.recall_at_k),
            percent(summary.exact_match),
        ));
    }
    table
}
