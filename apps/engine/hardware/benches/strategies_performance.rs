#![allow(clippy::unwrap_used, clippy::expect_used, reason = "benchmark setup")]
#![allow(
    missing_docs,
    reason = "criterion_group generates the undocumented harness functions"
)]

//! Scalar, SIMD, parallel and CUDA strategies at common embedding dimensions. The CUDA
//! strategy rows include the upload of query and candidates and the download of scores; the
//! batch_resident group scores a slab already on the device.
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::hint::black_box;

use piramid_hardware::compute::{strategies, DistanceKernels, ExecutionMode};

const COMPARED: [ExecutionMode; 4] = [
    ExecutionMode::Scalar,
    ExecutionMode::Simd,
    ExecutionMode::Parallel,
    ExecutionMode::Gpu,
];

/// Embedding dimensions of MiniLM, OpenAI small and ada, and OpenAI large.
const DIMS: [usize; 4] = [384, 768, 1536, 3072];

/// Candidate counts spanning one HNSW ef list up to a small flat collection.
const ROWS: [usize; 3] = [128, 1024, 8192];

/// Deterministic filler from a fixed LCG.
fn vectors(count: usize, dim: usize) -> Vec<f32> {
    let mut state = 0x2545_F491_4F6C_DD1Du64;
    (0..count * dim)
        .map(|_| {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1);
            // Maps the high bits of the LCG state onto the range -1 to 1.
            f32::from(((state >> 33) & 0xFFFF) as u16) / 32_768.0 - 1.0
        })
        .collect()
}

/// Install device 0 for the gpu mode when this build has a GPU backend and a device is present.
#[cfg(feature = "gpu-cuda")]
fn install_device() {
    use piramid_hardware::gpu::{BudgetSettings, GpuManager};
    static MANAGER: std::sync::OnceLock<Option<GpuManager>> = std::sync::OnceLock::new();
    MANAGER.get_or_init(|| {
        let settings = BudgetSettings {
            limit_bytes: None,
            reserve_bytes: 0,
            shares: None,
        };
        let manager = GpuManager::open(0, settings, 1).ok()?;
        strategies::install_gpu(&manager, 256).ok()?;
        Some(manager)
    });
}

#[cfg(not(feature = "gpu-cuda"))]
fn install_device() {}

/// Available strategies, resolved once, excluding Auto.
fn available() -> Vec<(&'static str, &'static dyn DistanceKernels)> {
    install_device();
    COMPARED
        .iter()
        .filter_map(|mode| strategies::for_mode(*mode).ok())
        .map(|kernels| (kernels.name(), kernels))
        .collect()
}

/// One query against one candidate.
fn pairwise(c: &mut Criterion) {
    let mut group = c.benchmark_group("pairwise/cosine");

    for dim in DIMS {
        let a = vectors(1, dim);
        let b = vectors(1, dim);
        group.throughput(Throughput::Elements(dim as u64));

        for (name, kernels) in available() {
            group.bench_with_input(BenchmarkId::new(name, dim), &dim, |bencher, _| {
                bencher.iter(|| kernels.cosine(black_box(&a), black_box(&b)));
            });
        }
    }
    group.finish();
}

/// One query against many candidates in a single batch call.
fn batch(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch/cosine");
    // Dimension is fixed; candidate count varies.
    let dim = 768;
    let query = vectors(1, dim);

    for rows in ROWS {
        let candidates = vectors(rows, dim);
        let mut out = vec![0.0f32; rows];
        group.throughput(Throughput::Elements((rows * dim) as u64));

        for (name, kernels) in available() {
            group.bench_with_input(BenchmarkId::new(name, rows), &rows, |bencher, _| {
                bencher.iter(|| {
                    kernels
                        .cosine_batch(black_box(&query), black_box(&candidates), dim, &mut out)
                        .unwrap();
                });
            });
        }
    }
    group.finish();
}

/// One query against a candidate slab uploaded once, scores left on the device.
#[cfg(feature = "gpu-cuda")]
fn batch_resident(c: &mut Criterion) {
    use piramid_hardware::gpu::kernels::distance::{DistanceLaunch, DistanceModule};
    use piramid_hardware::gpu::{Device, DeviceBuffer, Stream};

    let Ok(device) = Device::open(0) else {
        return;
    };
    let stream = Stream::new(&device).unwrap();
    let module = DistanceModule::compile(&device, 256).unwrap();
    let mut group = c.benchmark_group("batch_resident/cosine");
    let dim = 768;
    let query = vectors(1, dim);
    let norm: f32 = query.iter().map(|x| x * x).sum();
    let query_gpu = DeviceBuffer::from_host(&device, &query, &stream).unwrap();

    for rows in ROWS {
        let slab_gpu = DeviceBuffer::from_host(&device, &vectors(rows, dim), &stream).unwrap();
        let mut out_gpu = DeviceBuffer::<f32>::alloc(&device, rows).unwrap();
        group.throughput(Throughput::Elements((rows * dim) as u64));
        group.bench_with_input(BenchmarkId::new("cuda", rows), &rows, |bencher, _| {
            bencher.iter(|| {
                let launch = DistanceLaunch {
                    query: &query_gpu,
                    candidates: &slab_gpu,
                    out: &mut out_gpu,
                    dim,
                    rows,
                };
                module.cosine_batch(launch, norm, &stream).unwrap();
                stream.synchronize().unwrap();
            });
        });
    }
    group.finish();
}

#[cfg(not(feature = "gpu-cuda"))]
fn batch_resident(_c: &mut Criterion) {}

criterion_group!(benches, pairwise, batch, batch_resident);
criterion_main!(benches);
