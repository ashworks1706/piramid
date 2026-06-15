// Main application configuration struct: all sub-configurations for the collection, index, search, quantization, memory management, WAL, parallelism, and execution mode.

use serde::{Deserialize, Serialize};

use super::{
    CacheConfig, CollectionConfig, ExecutionMode, HardwareConfig, HardwareProfile, LimitsConfig,
    LoggingConfig, MemoryConfig, ParallelismConfig, QuantizationConfig, QuantizationStage,
    SearchConfig, WalConfig,
};
use crate::index::{AutoIndexConfig, IndexConfig};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub index: IndexConfig,
    pub quantization: QuantizationConfig,
    pub memory: MemoryConfig,
    pub wal: WalConfig,
    pub parallelism: ParallelismConfig,
    pub execution: ExecutionMode,
    #[serde(default)]
    pub hardware: HardwareConfig,
    pub search: SearchConfig,
    pub limits: LimitsConfig,
    #[serde(default)]
    pub cache: CacheConfig,
    #[serde(default)]
    pub logging: LoggingConfig,
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
            hardware: HardwareConfig::default(),
            search: SearchConfig::default(),
            limits: LimitsConfig::default(),
            cache: CacheConfig::default(),
            logging: LoggingConfig::default(),
        }
    }
}

impl AppConfig {
    pub fn validate(&self) -> Result<(), String> {
        if matches!(self.execution, ExecutionMode::Gpu) {
            return Err("EXECUTION_MODE gpu is not implemented".into());
        }
        if matches!(
            self.quantization.level,
            crate::config::QuantizationLevel::Int4 | crate::config::QuantizationLevel::Float16
        ) {
            return Err("Configured quantization level is not implemented".into());
        }
        if self.wal.enabled && self.wal.checkpoint_frequency == 0 {
            return Err("WAL checkpoint_frequency must be > 0 when WAL is enabled".into());
        }
        if self.search.filter_overfetch == 0 {
            return Err("SEARCH filter_overfetch must be >= 1".into());
        }
        if let Some(recall) = self.search.budget.recall_target {
            if !(0.0..=1.0).contains(&recall) {
                return Err("SEARCH budget recall_target must be between 0.0 and 1.0".into());
            }
        }
        if self.search.adaptive.min_ef == 0
            || self.search.adaptive.max_ef < self.search.adaptive.min_ef
        {
            return Err("SEARCH adaptive ef range is invalid".into());
        }
        if self.search.adaptive.min_nprobe == 0
            || self.search.adaptive.max_nprobe < self.search.adaptive.min_nprobe
        {
            return Err("SEARCH adaptive nprobe range is invalid".into());
        }
        if self.search.adaptive.min_filter_overfetch == 0
            || self.search.adaptive.max_filter_overfetch < self.search.adaptive.min_filter_overfetch
        {
            return Err("SEARCH adaptive filter_overfetch range is invalid".into());
        }
        if self.memory.use_mmap && self.memory.initial_mmap_size == 0 {
            return Err("MEMORY initial_mmap_size must be > 0 when mmap is enabled".into());
        }
        if let IndexConfig::Auto { auto, .. } = &self.index {
            validate_auto_index(auto)?;
        }
        if self.quantization.level == crate::config::QuantizationLevel::None
            && self.quantization.stage != QuantizationStage::Disabled
        {
            return Err("QUANTIZATION stage must be disabled when level is none".into());
        }
        if self.quantization.level == crate::config::QuantizationLevel::None
            && (self.quantization.storage_enabled
                || self.quantization.index_enabled
                || self.quantization.query_enabled
                || self.quantization.result_enabled)
        {
            return Err("QUANTIZATION enabled flags require a non-none level".into());
        }
        if self.hardware.gpu_enabled && matches!(self.execution, ExecutionMode::Scalar) {
            return Err("HARDWARE gpu_enabled conflicts with scalar execution mode".into());
        }
        Ok(())
    }

    pub fn to_collection_config(&self) -> CollectionConfig {
        CollectionConfig {
            index: self.index.clone(),
            search: self.search,
            quantization: self.quantization,
            memory: self.memory,
            wal: self.wal,
            parallelism: self.parallelism,
            execution: self.execution,
            hardware: self.hardware,
            limits: self.limits,
            cache: self.cache,
            logging: self.logging,
        }
    }

    /// Apply environment variable overrides to an existing config.
    pub fn apply_env_overrides(&mut self) -> Result<(), String> {
        if let Ok(val) = std::env::var("INDEX_TYPE") {
            self.index = match val.to_lowercase().as_str() {
                "flat" => IndexConfig::Flat {
                    metric: crate::metrics::Metric::Cosine,
                    mode: ExecutionMode::Auto,
                    search: self.search,
                },
                "hnsw" => IndexConfig::Hnsw {
                    m: 16,
                    m_max: 32,
                    ef_construction: 200,
                    ef_search: 200,
                    ml: 1.0 / (16.0_f32).ln(),
                    metric: crate::metrics::Metric::Cosine,
                    mode: ExecutionMode::Auto,
                    search: self.search,
                },
                "ivf" => IndexConfig::Ivf {
                    num_clusters: 256,
                    num_probes: 8,
                    max_iterations: 20,
                    metric: crate::metrics::Metric::Cosine,
                    mode: ExecutionMode::Auto,
                    search: self.search,
                },
                _ => return Err(format!("Invalid INDEX_TYPE '{val}'")),
            };
        }
        if matches!(self.index, IndexConfig::Auto { .. }) {
            let mut auto = self.index.auto_config();
            let mut changed = false;
            if let Ok(val) = std::env::var("INDEX_AUTO_FLAT_MAX_VECTORS") {
                auto.flat_max_vectors = parse_env::<usize>("INDEX_AUTO_FLAT_MAX_VECTORS", &val)?;
                changed = true;
            }
            if let Ok(val) = std::env::var("INDEX_AUTO_IVF_MAX_VECTORS") {
                auto.ivf_max_vectors = parse_env::<usize>("INDEX_AUTO_IVF_MAX_VECTORS", &val)?;
                changed = true;
            }
            if let Ok(val) = std::env::var("INDEX_AUTO_IVF_NUM_CLUSTERS") {
                auto.ivf_num_clusters =
                    Some(parse_env::<usize>("INDEX_AUTO_IVF_NUM_CLUSTERS", &val)?);
                changed = true;
            }
            if let Ok(val) = std::env::var("INDEX_AUTO_IVF_NUM_PROBES") {
                auto.ivf_num_probes = Some(parse_env::<usize>("INDEX_AUTO_IVF_NUM_PROBES", &val)?);
                changed = true;
            }
            if let Ok(val) = std::env::var("INDEX_AUTO_IVF_MAX_ITERATIONS") {
                auto.ivf_max_iterations =
                    parse_env::<usize>("INDEX_AUTO_IVF_MAX_ITERATIONS", &val)?;
                changed = true;
            }
            if let Ok(val) = std::env::var("INDEX_AUTO_HNSW_M") {
                auto.hnsw_m = parse_env::<usize>("INDEX_AUTO_HNSW_M", &val)?;
                changed = true;
            }
            if let Ok(val) = std::env::var("INDEX_AUTO_HNSW_EF_CONSTRUCTION") {
                auto.hnsw_ef_construction =
                    parse_env::<usize>("INDEX_AUTO_HNSW_EF_CONSTRUCTION", &val)?;
                changed = true;
            }
            if let Ok(val) = std::env::var("INDEX_AUTO_HNSW_EF_SEARCH") {
                auto.hnsw_ef_search = parse_env::<usize>("INDEX_AUTO_HNSW_EF_SEARCH", &val)?;
                changed = true;
            }
            if changed {
                let (metric, mode) = self.index.get_metric_and_mode();
                let search = self.index.search_config();
                self.index = IndexConfig::Auto {
                    metric,
                    mode,
                    search,
                    auto,
                };
            }
        }

        if let Ok(val) = std::env::var("HARDWARE_PROFILE") {
            self.hardware.profile = parse_hardware_profile(&val)?;
            self.apply_hardware_profile();
        }
        if let Ok(val) = std::env::var("HARDWARE_GPU_ENABLED") {
            self.hardware.gpu_enabled = parse_bool_env("HARDWARE_GPU_ENABLED", &val)?;
        }
        if let Ok(val) = std::env::var("HARDWARE_CPU_THREADS") {
            self.hardware.cpu_threads = Some(parse_env::<usize>("HARDWARE_CPU_THREADS", &val)?);
            if let Some(threads) = self.hardware.cpu_threads {
                self.parallelism = self.parallelism.with_num_threads(threads);
            }
        }
        if let Ok(val) = std::env::var("HARDWARE_MEMORY_BUDGET_BYTES") {
            self.hardware.memory_budget_bytes =
                Some(parse_env::<u64>("HARDWARE_MEMORY_BUDGET_BYTES", &val)?);
        }

        if let Ok(val) = std::env::var("WAL_ENABLED") {
            self.wal.enabled = parse_bool_env("WAL_ENABLED", &val)?;
        }
        if let Ok(val) = std::env::var("WAL_CHECKPOINT_FREQUENCY") {
            let freq = parse_env::<usize>("WAL_CHECKPOINT_FREQUENCY", &val)?;
            self.wal.checkpoint_frequency = freq.max(1);
        }
        if let Ok(val) = std::env::var("WAL_CHECKPOINT_INTERVAL_SECS") {
            let secs = parse_env::<u64>("WAL_CHECKPOINT_INTERVAL_SECS", &val)?;
            self.wal.checkpoint_interval_secs = Some(secs.max(1));
        }

        if let Ok(val) = std::env::var("MEMORY_USE_MMAP") {
            self.memory.use_mmap = parse_bool_env("MEMORY_USE_MMAP", &val)?;
        }
        if let Ok(val) = std::env::var("MEMORY_INITIAL_MMAP_MB") {
            let mb = parse_env::<usize>("MEMORY_INITIAL_MMAP_MB", &val)?;
            self.memory.initial_mmap_size = mb * 1024 * 1024;
        }

        if let Ok(val) = std::env::var("PARALLEL_SEARCH") {
            self.parallelism.parallel_search = parse_bool_env("PARALLEL_SEARCH", &val)?;
        }
        if let Ok(val) = std::env::var("NUM_THREADS") {
            let n = parse_env::<usize>("NUM_THREADS", &val)?;
            self.parallelism = self.parallelism.with_num_threads(n);
        }

        if let Ok(val) = std::env::var("EXECUTION_MODE") {
            self.execution = match val.to_lowercase().as_str() {
                "simd" => ExecutionMode::Simd,
                "scalar" => ExecutionMode::Scalar,
                "gpu" => ExecutionMode::Gpu,
                "parallel" => ExecutionMode::Parallel,
                "binary" => ExecutionMode::Binary,
                "jit" => ExecutionMode::Jit,
                _ => return Err(format!("Invalid EXECUTION_MODE '{val}'")),
            };
        }

        if let Ok(val) = std::env::var("SEARCH_FILTER_OVERFETCH")
            .or_else(|_| std::env::var("SEARCH_FILTER_EXPANSION"))
        {
            let factor = parse_env::<usize>("SEARCH_FILTER_OVERFETCH", &val)?;
            self.search.filter_overfetch = factor.max(1);
        }
        if let Ok(val) = std::env::var("SEARCH_LATENCY_BUDGET_MS") {
            self.search.budget.latency_budget_ms =
                Some(parse_env::<u64>("SEARCH_LATENCY_BUDGET_MS", &val)?);
        }
        if let Ok(val) = std::env::var("SEARCH_RECALL_TARGET") {
            self.search.budget.recall_target =
                Some(parse_env::<f32>("SEARCH_RECALL_TARGET", &val)?);
        }
        if let Ok(val) = std::env::var("SEARCH_ADAPTIVE_ENABLED") {
            self.search.adaptive.enabled = parse_bool_env("SEARCH_ADAPTIVE_ENABLED", &val)?;
        }

        if let Ok(val) = std::env::var("QUANTIZATION_STAGE") {
            self.quantization.stage = parse_quantization_stage(&val)?;
        }
        if let Ok(val) = std::env::var("QUANTIZATION_PRESERVE_RAW") {
            self.quantization.preserve_raw_vectors =
                parse_bool_env("QUANTIZATION_PRESERVE_RAW", &val)?;
        }
        if let Ok(val) = std::env::var("QUANTIZATION_STORAGE_ENABLED") {
            self.quantization.storage_enabled =
                parse_bool_env("QUANTIZATION_STORAGE_ENABLED", &val)?;
        }
        if let Ok(val) = std::env::var("QUANTIZATION_INDEX_ENABLED") {
            self.quantization.index_enabled = parse_bool_env("QUANTIZATION_INDEX_ENABLED", &val)?;
        }
        if let Ok(val) = std::env::var("QUANTIZATION_QUERY_ENABLED") {
            self.quantization.query_enabled = parse_bool_env("QUANTIZATION_QUERY_ENABLED", &val)?;
        }
        if let Ok(val) = std::env::var("QUANTIZATION_RESULT_ENABLED") {
            self.quantization.result_enabled = parse_bool_env("QUANTIZATION_RESULT_ENABLED", &val)?;
        }

        if let Ok(val) = std::env::var("LIMIT_MAX_VECTORS") {
            self.limits.max_vectors = Some(parse_env::<usize>("LIMIT_MAX_VECTORS", &val)?);
        }
        if let Ok(val) = std::env::var("LIMIT_MAX_BYTES") {
            self.limits.max_bytes = Some(parse_env::<u64>("LIMIT_MAX_BYTES", &val)?);
        }
        if let Ok(val) = std::env::var("LIMIT_MAX_VECTOR_BYTES") {
            self.limits.max_vector_bytes =
                Some(parse_env::<usize>("LIMIT_MAX_VECTOR_BYTES", &val)?);
        }

        if let Ok(val) = std::env::var("CACHE_ENABLED") {
            self.cache.enabled = parse_bool_env("CACHE_ENABLED", &val)?;
        }
        if let Ok(val) = std::env::var("CACHE_MAX_SIZE") {
            self.cache.max_size = parse_env::<usize>("CACHE_MAX_SIZE", &val)?;
        }
        if let Ok(val) = std::env::var("CACHE_TTL_SECONDS") {
            self.cache.ttl_seconds = Some(parse_env::<u64>("CACHE_TTL_SECONDS", &val)?);
        }
        if let Ok(val) = std::env::var("CACHE_MAX_BYTES") {
            self.cache.max_bytes = Some(parse_env::<u64>("CACHE_MAX_BYTES", &val)?);
        }
        if let Ok(val) = std::env::var("LOG_LEVEL") {
            self.logging.level = parse_log_level(&val)?;
        }
        if let Ok(val) = std::env::var("LOG_SEARCH") {
            self.logging.search = parse_bool_env("LOG_SEARCH", &val)?;
        }
        if let Ok(val) = std::env::var("LOG_INDEXING") {
            self.logging.indexing = parse_bool_env("LOG_INDEXING", &val)?;
        }
        if let Ok(val) = std::env::var("LOG_WRITES") {
            self.logging.writes = parse_bool_env("LOG_WRITES", &val)?;
        }
        if let Ok(val) = std::env::var("LOG_INFERENCE") {
            self.logging.inference = parse_bool_env("LOG_INFERENCE", &val)?;
        }
        Ok(())
    }

    pub fn apply_hardware_profile(&mut self) {
        match self.hardware.profile {
            HardwareProfile::Auto => {}
            HardwareProfile::CpuOnly => {
                self.hardware.gpu_enabled = false;
                self.execution = ExecutionMode::Auto;
            }
            HardwareProfile::Gpu => {
                self.hardware.gpu_enabled = true;
                self.execution = ExecutionMode::Gpu;
            }
            HardwareProfile::Memory8Gb => {
                self.hardware.memory_budget_bytes = Some(8 * 1024 * 1024 * 1024);
                self.cache.max_bytes = Some(512 * 1024 * 1024);
                self.memory.max_memory_per_collection = Some(2 * 1024 * 1024 * 1024);
            }
            HardwareProfile::Memory16Gb => {
                self.hardware.memory_budget_bytes = Some(16 * 1024 * 1024 * 1024);
                self.cache.max_bytes = Some(1024 * 1024 * 1024);
                self.memory.max_memory_per_collection = Some(4 * 1024 * 1024 * 1024);
            }
            HardwareProfile::Memory32Gb => {
                self.hardware.memory_budget_bytes = Some(32 * 1024 * 1024 * 1024);
                self.cache.max_bytes = Some(2 * 1024 * 1024 * 1024);
                self.memory.max_memory_per_collection = Some(8 * 1024 * 1024 * 1024);
            }
        }
    }

    pub fn from_env() -> Self {
        // Create a default configuration and then apply any overrides from environment variables.
        let mut cfg = AppConfig::default();
        cfg.apply_env_overrides()
            .expect("invalid application configuration environment override");
        cfg
    }
}

fn parse_env<T>(name: &str, value: &str) -> Result<T, String>
where
    T: std::str::FromStr,
{
    value
        .parse::<T>()
        .map_err(|_| format!("Invalid {name} value '{value}'"))
}

fn parse_bool_env(name: &str, value: &str) -> Result<bool, String> {
    match value.to_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Ok(true),
        "0" | "false" | "no" | "off" => Ok(false),
        _ => Err(format!("Invalid {name} value '{value}'")),
    }
}

fn validate_auto_index(auto: &AutoIndexConfig) -> Result<(), String> {
    if auto.flat_max_vectors == 0 {
        return Err("INDEX auto flat_max_vectors must be > 0".into());
    }
    if auto.ivf_max_vectors <= auto.flat_max_vectors {
        return Err("INDEX auto ivf_max_vectors must be greater than flat_max_vectors".into());
    }
    if auto.ivf_num_clusters == Some(0) {
        return Err("INDEX auto ivf_num_clusters must be > 0 when set".into());
    }
    if auto.ivf_num_probes == Some(0) {
        return Err("INDEX auto ivf_num_probes must be > 0 when set".into());
    }
    if auto.ivf_max_iterations == 0 {
        return Err("INDEX auto ivf_max_iterations must be > 0".into());
    }
    if auto.hnsw_m == 0 {
        return Err("INDEX auto hnsw_m must be > 0".into());
    }
    if auto.hnsw_ef_construction == 0 || auto.hnsw_ef_search == 0 {
        return Err("INDEX auto HNSW ef values must be > 0".into());
    }
    Ok(())
}

fn parse_hardware_profile(value: &str) -> Result<HardwareProfile, String> {
    match value.to_lowercase().as_str() {
        "auto" => Ok(HardwareProfile::Auto),
        "cpu-only" | "cpu" => Ok(HardwareProfile::CpuOnly),
        "gpu" => Ok(HardwareProfile::Gpu),
        "8gb" | "memory-8gb" => Ok(HardwareProfile::Memory8Gb),
        "16gb" | "memory-16gb" => Ok(HardwareProfile::Memory16Gb),
        "32gb" | "memory-32gb" => Ok(HardwareProfile::Memory32Gb),
        _ => Err(format!("Invalid HARDWARE_PROFILE '{value}'")),
    }
}

fn parse_quantization_stage(value: &str) -> Result<QuantizationStage, String> {
    match value.to_lowercase().as_str() {
        "disabled" | "none" => Ok(QuantizationStage::Disabled),
        "storage" => Ok(QuantizationStage::Storage),
        "index" => Ok(QuantizationStage::Index),
        "query-pre-search" | "query_pre_search" | "pre-search" | "pre" => {
            Ok(QuantizationStage::QueryPreSearch)
        }
        "result-post-search" | "result_post_search" | "post-search" | "post" => {
            Ok(QuantizationStage::ResultPostSearch)
        }
        _ => Err(format!("Invalid QUANTIZATION_STAGE '{value}'")),
    }
}

fn parse_log_level(value: &str) -> Result<crate::config::LogLevel, String> {
    match value.to_lowercase().as_str() {
        "error" => Ok(crate::config::LogLevel::Error),
        "warn" => Ok(crate::config::LogLevel::Warn),
        "info" => Ok(crate::config::LogLevel::Info),
        "debug" => Ok(crate::config::LogLevel::Debug),
        "trace" => Ok(crate::config::LogLevel::Trace),
        _ => Err(format!("Invalid LOG_LEVEL '{value}'")),
    }
}
