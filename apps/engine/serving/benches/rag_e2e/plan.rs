//! What one run of the end-to-end benchmark measures: its arms and the settings read from the
//! environment.

use std::path::PathBuf;

use piramid_core::config::EmbeddingConfig;
use serde::Serialize;

/// One way of answering a question.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Arm {
    /// The question alone, with no retrieval.
    ClosedBook,
    /// Passages from a separate piramid server over HTTP, placed before prefill.
    BeforePrefillHttp,
    /// Passages from an in-process collection scored on the host, placed before prefill.
    BeforePrefillHost,
    /// Passages from an in-process collection scored on the GPU, placed before prefill.
    BeforePrefillDevice,
}

impl Arm {
    /// Every arm, in report order.
    pub const ALL: [Arm; 4] = [
        Arm::ClosedBook,
        Arm::BeforePrefillHttp,
        Arm::BeforePrefillHost,
        Arm::BeforePrefillDevice,
    ];

    /// The name the arm is selected and reported by.
    pub fn as_str(self) -> &'static str {
        match self {
            Arm::ClosedBook => "closed-book",
            Arm::BeforePrefillHttp => "before-prefill-http",
            Arm::BeforePrefillHost => "before-prefill-host",
            Arm::BeforePrefillDevice => "before-prefill-device",
        }
    }

    /// The arm with name.
    pub fn parse(name: &str) -> Result<Arm, String> {
        Arm::ALL
            .into_iter()
            .find(|arm| arm.as_str() == name)
            .ok_or_else(|| {
                let known: Vec<&str> = Arm::ALL.iter().map(|arm| arm.as_str()).collect();
                format!(
                    "PIRAMID_BENCH_ARMS: unknown arm {name}, expected one of {}",
                    known.join(", ")
                )
            })
    }
}

/// Arms run when PIRAMID_BENCH_ARMS is unset.
pub const DEFAULT_ARMS: &str = "closed-book,before-prefill-host";

/// Settings of one run.
#[derive(Debug, Clone, PartialEq)]
pub struct Plan {
    /// Checkpoint directory of the model.
    pub model: PathBuf,
    /// Question file.
    pub dataset: PathBuf,
    /// Device the model loads onto: cpu or cuda:N.
    pub device: String,
    /// Embedding provider for passages and questions.
    pub embedding: EmbeddingConfig,
    /// Questions read from the file. None reads every question.
    pub questions: Option<usize>,
    /// Passages retrieved per question.
    pub k: usize,
    /// Arms run, in the order given.
    pub arms: Vec<Arm>,
    /// Where the results are written.
    pub out: PathBuf,
    /// The search/text endpoint of the server the HTTP arm queries.
    pub search_url: Option<String>,
    /// Tokens generated per question.
    pub max_new_tokens: usize,
    /// Byte budget of the key/value cache.
    pub kv_cache_bytes: u64,
    /// Questions run once per arm before recording starts.
    pub warmup: usize,
}

/// Tokens generated per question when PIRAMID_BENCH_MAX_NEW_TOKENS is unset.
pub const DEFAULT_MAX_NEW_TOKENS: usize = 32;
/// Key/value cache budget when PIRAMID_BENCH_KV_CACHE_BYTES is unset.
pub const DEFAULT_KV_CACHE_BYTES: u64 = 2 << 30;
/// Passages retrieved when PIRAMID_BENCH_K is unset.
pub const DEFAULT_K: usize = 5;

impl Plan {
    /// Read the settings through lookup, writing to default_out when PIRAMID_BENCH_OUT is unset.
    ///
    /// The embedding response cache is turned off so every question is embedded.
    pub fn from_lookup(
        lookup: impl Fn(&str) -> Option<String>,
        default_out: PathBuf,
    ) -> Result<Plan, String> {
        let required = |name: &str| {
            lookup(name)
                .filter(|value| !value.trim().is_empty())
                .ok_or_else(|| format!("{name} is required"))
        };
        let mut embedding: EmbeddingConfig =
            serde_json::from_str(&required("PIRAMID_BENCH_EMBEDDING")?)
                .map_err(|e| format!("PIRAMID_BENCH_EMBEDDING: {e}"))?;
        embedding.cache.enabled = false;
        embedding
            .validate()
            .map_err(|e| format!("PIRAMID_BENCH_EMBEDDING: {e}"))?;

        let arms = lookup("PIRAMID_BENCH_ARMS")
            .unwrap_or_else(|| DEFAULT_ARMS.to_string())
            .split(',')
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .map(Arm::parse)
            .collect::<Result<Vec<_>, _>>()?;
        if arms.is_empty() {
            return Err("PIRAMID_BENCH_ARMS names no arm".to_string());
        }
        if let Some(arm) = arms
            .iter()
            .enumerate()
            .find(|(index, arm)| arms[..*index].contains(arm))
            .map(|(_, arm)| arm)
        {
            return Err(format!(
                "PIRAMID_BENCH_ARMS names {} more than once",
                arm.as_str()
            ));
        }

        let k = number(&lookup, "PIRAMID_BENCH_K")?.unwrap_or(DEFAULT_K);
        if k == 0 {
            return Err("PIRAMID_BENCH_K must be >= 1".to_string());
        }
        let questions = number(&lookup, "PIRAMID_BENCH_QUESTIONS")?;
        if questions == Some(0) {
            return Err("PIRAMID_BENCH_QUESTIONS must be >= 1".to_string());
        }
        let max_new_tokens =
            number(&lookup, "PIRAMID_BENCH_MAX_NEW_TOKENS")?.unwrap_or(DEFAULT_MAX_NEW_TOKENS);
        if max_new_tokens == 0 {
            return Err("PIRAMID_BENCH_MAX_NEW_TOKENS must be >= 1".to_string());
        }

        Ok(Plan {
            model: PathBuf::from(required("PIRAMID_BENCH_MODEL")?),
            dataset: PathBuf::from(required("PIRAMID_BENCH_DATASET")?),
            device: lookup("PIRAMID_BENCH_DEVICE").unwrap_or_else(|| "cpu".to_string()),
            embedding,
            questions,
            k,
            arms,
            out: lookup("PIRAMID_BENCH_OUT").map_or(default_out, PathBuf::from),
            search_url: lookup("PIRAMID_BENCH_SEARCH_URL"),
            max_new_tokens,
            kv_cache_bytes: number(&lookup, "PIRAMID_BENCH_KV_CACHE_BYTES")?
                .unwrap_or(DEFAULT_KV_CACHE_BYTES),
            warmup: number(&lookup, "PIRAMID_BENCH_WARMUP")?.unwrap_or(1),
        })
    }

    /// Refuse an arm this build or these settings cannot run. gpu_compiled is whether the build
    /// has the gpu-cuda feature.
    pub fn check_arms(&self, gpu_compiled: bool) -> Result<(), String> {
        for arm in &self.arms {
            match arm {
                Arm::BeforePrefillDevice if !gpu_compiled => {
                    return Err(format!(
                        "{} needs a build with the gpu-cuda feature",
                        arm.as_str()
                    ))
                }
                Arm::BeforePrefillHttp => {
                    let url = self.search_url.as_deref().ok_or_else(|| {
                        format!("{} needs PIRAMID_BENCH_SEARCH_URL", arm.as_str())
                    })?;
                    collection_url(url)?;
                }
                _ => {}
            }
        }
        Ok(())
    }
}

/// Parse the variable name as a non-negative integer. None when unset.
fn number<T: std::str::FromStr>(
    lookup: &impl Fn(&str) -> Option<String>,
    name: &str,
) -> Result<Option<T>, String>
where
    T::Err: std::fmt::Display,
{
    lookup(name)
        .map(|value| {
            value
                .trim()
                .parse::<T>()
                .map_err(|e| format!("{name}: {value} is not a whole number: {e}"))
        })
        .transpose()
}

/// The collection URL a search/text URL belongs to.
pub fn collection_url(search_url: &str) -> Result<&str, String> {
    search_url
        .strip_suffix("/search/text")
        .filter(|base| base.contains("/api/collections/"))
        .ok_or_else(|| {
            format!(
                "PIRAMID_BENCH_SEARCH_URL {search_url} is not an /api/collections/NAME/search/text \
                 endpoint"
            )
        })
}
