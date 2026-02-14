use serde::{Serialize, Deserialize};

use super::{
    CollectionConfig, SearchConfig, QuantizationConfig, MemoryConfig, WalConfig,
        ParallelismConfig, ExecutionMode, LimitsConfig,
};
use crate::index::IndexConfig;

// Main application configuration struct that encompasses all sub-configurations for the collection, index, search, quantization, memory management, WAL, parallelism, and execution mode. This struct can be easily serialized/deserialized from JSON or other formats for configuration files or environment variable overrides.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub index: IndexConfig,
    pub quantization: QuantizationConfig,
    pub memory: MemoryConfig,
    pub wal: WalConfig,
    pub parallelism: ParallelismConfig,
    pub execution: ExecutionMode,
    pub search: SearchConfig,
    pub limits: LimitsConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            index: IndexConfig::default(),
            quantization: QuantizationConfig::default(),
            memory: MemoryConfig::default(),
            wal: WalConfig::default(),
            parallelism: ParallelismConfig::default(),
            execution: ExecutionMode::Auto,
            search: SearchConfig::default(),
            limits: LimitsConfig::default(),
        }
    }
}

impl AppConfig {
    pub fn validate(&self) -> Result<(), String> {
        if self.wal.enabled {
            if self.wal.checkpoint_frequency == 0 {
                return Err("WAL checkpoint_frequency must be > 0 when WAL is enabled".into());
            }
        }
        if self.search.filter_overfetch == 0 {
            return Err("SEARCH filter_overfetch must be >= 1".into());
        }
        if self.memory.use_mmap && self.memory.initial_mmap_size == 0 {
            return Err("MEMORY initial_mmap_size must be > 0 when mmap is enabled".into());
        }
        Ok(())
    }

    pub fn to_collection_config(&self) -> CollectionConfig {
        CollectionConfig {
            index: self.index.clone(),
            search: self.search.clone(),
            quantization: self.quantization.clone(),
            memory: self.memory.clone(),
            wal: self.wal.clone(),
            parallelism: self.parallelism.clone(),
            execution: self.execution,
            limits: self.limits.clone(),
        }
    }

    /// Apply environment variable overrides to an existing config.
    pub fn apply_env_overrides(&mut self) {
        // Check for environment variables that can override the default configuration values. This allows users to configure the application using environment variables without needing to modify configuration files. Each variable is checked and parsed, and if valid, it updates the corresponding configuration field.
        if let Ok(val) = std::env::var("INDEX_TYPE") {
            self.index = match val.to_lowercase().as_str() {
                "flat" => IndexConfig::Flat {
                    metric: crate::metrics::Metric::Cosine,
                    mode: ExecutionMode::Auto,
                    search: self.search.clone(),
                },
                "hnsw" => IndexConfig::Hnsw {
                    m: 16,
                    m_max: 32,
                    ef_construction: 200,
                    ef_search: 200,
                    ml: 1.0 / (16.0_f32).ln(),
                    metric: crate::metrics::Metric::Cosine,
                    mode: ExecutionMode::Auto,
                    search: self.search.clone(),
                },
                "ivf" => IndexConfig::Ivf {
                    num_clusters: 256,
                    num_probes: 8,
                    max_iterations: 20,
                    metric: crate::metrics::Metric::Cosine,
                    mode: ExecutionMode::Auto,
                    search: self.search.clone(),
                },
                _ => self.index.clone(),
            };
        }

        if let Ok(val) = std::env::var("WAL_ENABLED") {
            self.wal.enabled = val == "1" || val.eq_ignore_ascii_case("true");
        }
        if let Ok(val) = std::env::var("WAL_CHECKPOINT_FREQUENCY") {
            if let Ok(freq) = val.parse::<usize>() {
                self.wal.checkpoint_frequency = freq.max(1);
            }
        }
        if let Ok(val) = std::env::var("WAL_CHECKPOINT_INTERVAL_SECS") {
            if let Ok(secs) = val.parse::<u64>() {
                self.wal.checkpoint_interval_secs = Some(secs.max(1));
            }
        }

        if let Ok(val) = std::env::var("MEMORY_USE_MMAP") {
            self.memory.use_mmap = val == "1" || val.eq_ignore_ascii_case("true");
        }
        if let Ok(val) = std::env::var("MEMORY_INITIAL_MMAP_MB") {
            if let Ok(mb) = val.parse::<usize>() {
                self.memory.initial_mmap_size = mb * 1024 * 1024;
            }
        }

        if let Ok(val) = std::env::var("PARALLEL_SEARCH") {
            self.parallelism.parallel_search = val == "1" || val.eq_ignore_ascii_case("true");
        }
        if let Ok(val) = std::env::var("NUM_THREADS") {
            if let Ok(n) = val.parse::<usize>() {
                self.parallelism = self.parallelism.with_num_threads(n);
            }
        }

        if let Ok(val) = std::env::var("EXECUTION_MODE") {
            self.execution = match val.to_lowercase().as_str() {
                "simd" => ExecutionMode::Simd,
                "scalar" => ExecutionMode::Scalar,
                "gpu" => ExecutionMode::Gpu,
                "parallel" => ExecutionMode::Parallel,
                "binary" => ExecutionMode::Binary,
                "jit" => ExecutionMode::Jit,
                _ => ExecutionMode::Auto,
            };
        }

        if let Ok(val) = std::env::var("SEARCH_FILTER_OVERFETCH")
            .or_else(|_| std::env::var("SEARCH_FILTER_EXPANSION"))
        {
            if let Ok(factor) = val.parse::<usize>() {
                self.search.filter_overfetch = factor.max(1);
            }
        }

        if let Ok(val) = std::env::var("LIMIT_MAX_VECTORS") {
            if let Ok(v) = val.parse::<usize>() {
                self.limits.max_vectors = Some(v);
            }
        }
        if let Ok(val) = std::env::var("LIMIT_MAX_BYTES") {
            if let Ok(v) = val.parse::<u64>() {
                self.limits.max_bytes = Some(v);
            }
        }
        if let Ok(val) = std::env::var("LIMIT_MAX_VECTOR_BYTES") {
            if let Ok(v) = val.parse::<usize>() {
                self.limits.max_vector_bytes = Some(v);
            }
        }
    }

    pub fn from_env() -> Self {
        // Create a default configuration and then apply any overrides from environment variables. This allows for flexible configuration of the application through environment variables while still providing sensible defaults.
        let mut cfg = AppConfig::default();
        cfg.apply_env_overrides();
        cfg
    }
}
