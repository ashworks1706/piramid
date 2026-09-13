//! Renders /api/metrics JSON into the Prometheus text exposition format.

use piramid_core::observability::prometheus::{MetricType, Registry};

use crate::services::api::{
    GpuBudgetResponse, GpuMetricsResponse, HostMetricsResponse, InferenceMetricsResponse,
    MetricsResponse,
};

/// Render a metrics snapshot in the Prometheus text format.
pub fn render(metrics: &MetricsResponse) -> String {
    let mut registry = Registry::new();

    registry.metric(
        "piramid_collections_total",
        "Number of collections currently loaded.",
        MetricType::Gauge,
        metrics.total_collections as f64,
    );
    registry.metric(
        "piramid_vectors_total",
        "Number of vectors across all loaded collections.",
        MetricType::Gauge,
        metrics.total_vectors as f64,
    );

    let by_collection = |extract: fn(&crate::services::api::CollectionMetrics) -> Option<f64>| {
        metrics
            .collections
            .iter()
            .filter_map(|c| extract(c).map(|value| (vec![("collection", c.name.clone())], value)))
            .collect::<Vec<_>>()
    };

    registry.metric_family(
        "piramid_collection_vectors",
        "Vectors in a collection.",
        MetricType::Gauge,
        by_collection(|c| Some(c.vector_count as f64)),
    );
    registry.metric_family(
        "piramid_collection_memory_bytes",
        "Approximate resident bytes for a collection.",
        MetricType::Gauge,
        by_collection(|c| Some(c.memory_usage_bytes as f64)),
    );
    registry.metric_family(
        "piramid_collection_insert_latency_ms",
        "Mean insert latency in milliseconds.",
        MetricType::Gauge,
        by_collection(|c| c.insert_latency_ms.map(f64::from)),
    );
    registry.metric_family(
        "piramid_collection_search_latency_ms",
        "Mean search latency in milliseconds.",
        MetricType::Gauge,
        by_collection(|c| c.search_latency_ms.map(f64::from)),
    );
    registry.metric_family(
        "piramid_collection_lock_read_ms",
        "Mean time waiting for a collection read lock, in milliseconds.",
        MetricType::Gauge,
        by_collection(|c| c.lock_read_ms.map(f64::from)),
    );
    registry.metric_family(
        "piramid_collection_lock_write_ms",
        "Mean time waiting for a collection write lock, in milliseconds.",
        MetricType::Gauge,
        by_collection(|c| c.lock_write_ms.map(f64::from)),
    );

    registry.metric_family(
        "piramid_wal_size_bytes",
        "Write-ahead log size for a collection.",
        MetricType::Gauge,
        metrics
            .wal_stats
            .iter()
            .filter_map(|w| {
                w.wal_size_bytes
                    .map(|bytes| (vec![("collection", w.collection.clone())], bytes as f64))
            })
            .collect::<Vec<_>>(),
    );
    registry.metric_family(
        "piramid_wal_checkpoint_age_seconds",
        "Seconds since a collection last checkpointed.",
        MetricType::Gauge,
        metrics
            .wal_stats
            .iter()
            .filter_map(|w| {
                w.checkpoint_age_secs
                    .map(|age| (vec![("collection", w.collection.clone())], age as f64))
            })
            .collect::<Vec<_>>(),
    );

    registry.metric(
        "piramid_embedding_requests_total",
        "Embedding requests issued to the provider.",
        MetricType::Counter,
        metrics.embedding.requests as f64,
    );
    registry.metric(
        "piramid_embedding_texts_total",
        "Texts submitted for embedding.",
        MetricType::Counter,
        metrics.embedding.texts as f64,
    );
    registry.optional_metric(
        "piramid_embedding_tokens_total",
        "Tokens reported by the embedding provider.",
        MetricType::Counter,
        metrics.embedding.total_tokens.map(|tokens| tokens as f64),
    );
    registry.optional_metric(
        "piramid_embedding_latency_ms",
        "Mean embedding request latency in milliseconds.",
        MetricType::Gauge,
        metrics.embedding.avg_latency_ms.map(f64::from),
    );

    render_host(&mut registry, &metrics.host);
    render_gpus(&mut registry, &metrics.gpus);
    if let Some(budget) = &metrics.gpu_budget {
        render_gpu_budget(&mut registry, budget);
    }
    if let Some(inference) = &metrics.inference {
        render_inference(&mut registry, inference);
    }

    registry.render()
}

/// Write the host readings, leaving out each one the server could not measure.
pub fn render_host(registry: &mut Registry, host: &HostMetricsResponse) {
    registry.optional_metric(
        "piramid_host_cpu_percent",
        "Processor use across every logical CPU of the host, from 0 to 100.",
        MetricType::Gauge,
        host.cpu_percent.map(f64::from),
    );
    registry.optional_metric(
        "piramid_host_memory_used_bytes",
        "Physical memory in use on the host.",
        MetricType::Gauge,
        host.memory_used_bytes.map(|bytes| bytes as f64),
    );
    registry.optional_metric(
        "piramid_host_memory_total_bytes",
        "Physical memory installed on the host.",
        MetricType::Gauge,
        host.memory_total_bytes.map(|bytes| bytes as f64),
    );
    registry.optional_metric(
        "piramid_process_cpu_percent",
        "Processor use of the server process as a share of every logical CPU of the host, from 0 to 100.",
        MetricType::Gauge,
        host.process_cpu_percent.map(f64::from),
    );
    registry.optional_metric(
        "piramid_process_resident_memory_bytes",
        "Resident memory of the server process.",
        MetricType::Gauge,
        host.process_resident_bytes.map(|bytes| bytes as f64),
    );
}

/// Write the device memory budget and the capacity and use of each pool.
fn render_gpu_budget(registry: &mut Registry, budget: &GpuBudgetResponse) {
    registry.metric(
        "piramid_gpu_budget_usable_bytes",
        "Device memory the budget covers after the reserve.",
        MetricType::Gauge,
        budget.usable_bytes as f64,
    );
    let by_pool = |extract: fn(&crate::services::api::GpuPoolResponse) -> u64| {
        budget
            .pools
            .iter()
            .map(|pool| (vec![("pool", pool.pool.to_string())], extract(pool) as f64))
            .collect::<Vec<_>>()
    };
    registry.metric_family(
        "piramid_gpu_pool_capacity_bytes",
        "Device memory a pool may hold.",
        MetricType::Gauge,
        by_pool(|pool| pool.capacity_bytes),
    );
    registry.metric_family(
        "piramid_gpu_pool_used_bytes",
        "Device memory reserved in a pool.",
        MetricType::Gauge,
        by_pool(|pool| pool.used_bytes),
    );
}

fn render_inference(registry: &mut Registry, inference: &InferenceMetricsResponse) {
    let counters: [(&str, &str, u64); 8] = [
        (
            "piramid_inference_requests_admitted_total",
            "Generation requests accepted into the queue.",
            inference.requests_admitted,
        ),
        (
            "piramid_inference_requests_finished_total",
            "Generation requests that ended with a finish reason.",
            inference.requests_finished,
        ),
        (
            "piramid_inference_requests_failed_total",
            "Generation requests that ended with an error.",
            inference.requests_failed,
        ),
        (
            "piramid_inference_prompt_tokens_total",
            "Prompt tokens across admitted requests.",
            inference.prompt_tokens,
        ),
        (
            "piramid_inference_cached_prompt_tokens_total",
            "Prompt tokens served from shared key/value pages.",
            inference.cached_prompt_tokens,
        ),
        (
            "piramid_inference_generated_tokens_total",
            "Tokens generated.",
            inference.generated_tokens,
        ),
        (
            "piramid_inference_preemptions_total",
            "Sequences preempted for recompute.",
            inference.preemptions,
        ),
        (
            "piramid_kv_evictions_total",
            "Prefix pages evicted to make room.",
            inference.kv_evictions,
        ),
    ];
    for (name, help, value) in counters {
        registry.metric(name, help, MetricType::Counter, value as f64);
    }
    let gauges: [(&str, &str, u64); 6] = [
        (
            "piramid_inference_queue_depth",
            "Generation requests waiting for admission.",
            inference.queue_depth,
        ),
        (
            "piramid_inference_running",
            "Sequences being generated.",
            inference.running,
        ),
        (
            "piramid_inference_batch_size",
            "Sequences in the most recent forward step.",
            inference.last_batch_size,
        ),
        (
            "piramid_kv_blocks",
            "Pages in the key/value pool.",
            inference.kv_blocks_total,
        ),
        (
            "piramid_kv_blocks_used",
            "Key/value pages held by a sequence.",
            inference.kv_blocks_used,
        ),
        (
            "piramid_kv_blocks_cached",
            "Free key/value pages still carrying a reusable prefix.",
            inference.kv_blocks_cached,
        ),
    ];
    for (name, help, value) in gauges {
        registry.metric(name, help, MetricType::Gauge, value as f64);
    }
    let averages: [(&str, &str, Option<f32>); 5] = [
        (
            "piramid_inference_time_to_first_token_ms",
            "Mean time from admission to the first token, in milliseconds.",
            inference.avg_time_to_first_token_ms,
        ),
        (
            "piramid_inference_decode_tokens_per_second",
            "Decode tokens per second.",
            inference.decode_tokens_per_second,
        ),
        (
            "piramid_inference_decode_step_ms",
            "Mean decode step duration, in milliseconds.",
            inference.avg_decode_step_ms,
        ),
        (
            "piramid_inference_prefill_tokens_per_second",
            "Prefill tokens per second.",
            inference.prefill_tokens_per_second,
        ),
        (
            "piramid_kv_prefix_hit_ratio",
            "Share of looked-up prompt tokens served from shared pages.",
            inference.prefix_hit_rate,
        ),
    ];
    for (name, help, value) in averages {
        registry.optional_metric(name, help, MetricType::Gauge, value.map(f64::from));
    }
}

/// Write the GPU readings, one sample per device labelled by its index, leaving out each one the
/// server could not measure.
pub fn render_gpus(registry: &mut Registry, gpus: &[GpuMetricsResponse]) {
    let by_device = |extract: fn(&GpuMetricsResponse) -> Option<f64>| {
        gpus.iter()
            .filter_map(|gpu| {
                extract(gpu).map(|value| (vec![("gpu", gpu.index.to_string())], value))
            })
            .collect::<Vec<_>>()
    };
    registry.metric_family(
        "piramid_gpu_memory_used_bytes",
        "Device memory in use on a GPU.",
        MetricType::Gauge,
        by_device(|gpu| gpu.memory_used_bytes.map(|bytes| bytes as f64)),
    );
    registry.metric_family(
        "piramid_gpu_memory_total_bytes",
        "Device memory installed on a GPU.",
        MetricType::Gauge,
        by_device(|gpu| gpu.memory_total_bytes.map(|bytes| bytes as f64)),
    );
    registry.metric_family(
        "piramid_gpu_utilization_percent",
        "Share of the last sample period during which a kernel ran on a GPU, from 0 to 100.",
        MetricType::Gauge,
        by_device(|gpu| gpu.utilization_percent.map(f64::from)),
    );
    registry.metric_family(
        "piramid_gpu_temperature_celsius",
        "Temperature of a GPU die in degrees Celsius.",
        MetricType::Gauge,
        by_device(|gpu| gpu.temperature_celsius.map(f64::from)),
    );
}
