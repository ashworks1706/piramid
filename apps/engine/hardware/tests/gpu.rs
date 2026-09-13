//! Device runtime round-trips and distance kernel parity with the scalar strategy. Needs a CUDA
//! device; run with just test-gpu.
#![cfg(feature = "gpu-cuda")]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "assertions in tests"
)]

use piramid_hardware::compute::strategies::{for_mode, install_gpu};
use piramid_hardware::compute::ExecutionMode;
use piramid_hardware::gpu::kernels::distance::{DistanceLaunch, DistanceModule};
use piramid_hardware::gpu::{BudgetSettings, Device, DeviceBuffer, GpuManager, Stream};

fn device() -> Device {
    Device::open(0).expect("CUDA device 0")
}

fn filler(count: usize, seed: u64) -> Vec<f32> {
    let mut state = seed;
    (0..count)
        .map(|_| {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            f32::from(((state >> 33) & 0xFFFF) as u16) / 32_768.0 - 1.0
        })
        .collect()
}

#[test]
#[ignore = "needs a CUDA device"]
fn a_buffer_round_trips_through_the_device() {
    let device = device();
    let stream = Stream::new(&device).unwrap();
    let host = filler(10_000, 7);
    let buffer = DeviceBuffer::from_host(&device, &host, &stream).unwrap();
    assert_eq!(buffer.to_host(&stream).unwrap(), host);
    assert!(device.available_memory_bytes().unwrap() > 0);
}

#[test]
#[ignore = "needs a CUDA device"]
fn a_mismatched_host_slice_is_refused() {
    let device = device();
    let stream = Stream::default_for(&device);
    let mut buffer = DeviceBuffer::<f32>::alloc(&device, 4).unwrap();
    assert!(buffer.copy_from_host(&[1.0; 3], &stream).is_err());
}

#[test]
#[ignore = "needs a CUDA device"]
fn device_scores_match_scalar() {
    let device = device();
    let stream = Stream::new(&device).unwrap();
    let module = DistanceModule::compile(&device, 256).unwrap();
    let scalar = for_mode(ExecutionMode::Scalar).unwrap();

    for (dim, rows) in [(3, 1), (384, 1000), (768, 4099)] {
        let query = filler(dim, 11);
        let slab = filler(dim * rows, 13);
        let query_gpu = DeviceBuffer::from_host(&device, &query, &stream).unwrap();
        let slab_gpu = DeviceBuffer::from_host(&device, &slab, &stream).unwrap();
        let mut out_gpu = DeviceBuffer::<f32>::alloc(&device, rows).unwrap();
        let mut expected = vec![0.0f32; rows];

        let norm: f32 = query.iter().map(|x| x * x).sum();
        for name in ["cosine", "dot", "euclidean"] {
            let launch = DistanceLaunch {
                query: &query_gpu,
                candidates: &slab_gpu,
                out: &mut out_gpu,
                dim,
                rows,
            };
            match name {
                "cosine" => {
                    module.cosine_batch(launch, norm, &stream).unwrap();
                    scalar.cosine_batch(&query, &slab, dim, &mut expected)
                }
                "dot" => {
                    module.dot_batch(launch, &stream).unwrap();
                    scalar.dot_batch(&query, &slab, dim, &mut expected)
                }
                _ => {
                    module.euclidean_batch(launch, &stream).unwrap();
                    scalar.euclidean_batch(&query, &slab, dim, &mut expected)
                }
            }
            .unwrap();
            let got = out_gpu.to_host(&stream).unwrap();
            let worst = got
                .iter()
                .zip(&expected)
                .map(|(a, b)| (a - b).abs() / b.abs().max(1.0))
                .fold(0.0f32, f32::max);
            assert!(
                worst < 1e-5,
                "{name} dim {dim} rows {rows}: deviation {worst}"
            );
        }
    }
}

#[test]
#[ignore = "needs a CUDA device"]
fn device_top_k_matches_a_host_sort() {
    let device = device();
    let stream = Stream::new(&device).unwrap();
    let module = DistanceModule::compile(&device, 256).unwrap();

    for (rows, k) in [(5, 10), (1000, 10), (100_000, 64), (300_000, 1024)] {
        let scores = filler(rows, rows as u64);
        let scores_gpu = DeviceBuffer::from_host(&device, &scores, &stream).unwrap();
        let top = module.top_k(&scores_gpu, rows, k, &stream).unwrap();
        let got_scores = top.scores.to_host(&stream).unwrap();
        let got_indices = top.indices.to_host(&stream).unwrap();

        let mut order: Vec<usize> = (0..rows).collect();
        order.sort_by(|&a, &b| scores[b].total_cmp(&scores[a]));
        let taken = k.min(rows);
        for slot in 0..taken {
            assert_eq!(
                got_scores[slot], scores[order[slot]],
                "rows {rows} k {k} slot {slot}"
            );
            assert_eq!(scores[got_indices[slot] as usize], got_scores[slot]);
        }
        assert!(got_indices[taken..].iter().all(|&index| index == u32::MAX));
    }
}

#[test]
#[ignore = "needs a CUDA device"]
fn the_gpu_mode_resolves_to_the_cuda_strategy_and_matches_scalar() {
    assert!(
        for_mode(ExecutionMode::Gpu).is_err(),
        "no GPU is installed yet"
    );
    let manager = GpuManager::open(
        0,
        BudgetSettings {
            limit_bytes: None,
            reserve_bytes: 0,
            shares: None,
        },
        1,
    )
    .unwrap();
    install_gpu(&manager, 256).unwrap();
    let gpu = for_mode(ExecutionMode::Gpu).unwrap();
    let scalar = for_mode(ExecutionMode::Scalar).unwrap();
    assert_eq!(gpu.name(), "cuda");
    let (dim, rows) = (384, 257);
    let query = filler(dim, 3);
    let slab = filler(dim * rows, 5);
    let mut got = vec![0.0f32; rows];
    let mut expected = vec![0.0f32; rows];
    gpu.cosine_batch(&query, &slab, dim, &mut got).unwrap();
    scalar
        .cosine_batch(&query, &slab, dim, &mut expected)
        .unwrap();
    for (a, b) in got.iter().zip(&expected) {
        assert!((a - b).abs() < 1e-5);
    }
    let pair = gpu.cosine(&query, &slab[..dim]).unwrap();
    assert!((pair - expected[0]).abs() < 1e-5);
    let squared = gpu.euclidean_squared(&query, &slab[..dim]).unwrap();
    let reference = scalar.euclidean_squared(&query, &slab[..dim]).unwrap();
    assert!((squared - reference).abs() / reference.max(1.0) < 1e-5);
    assert!(gpu.cosine(&query, &slab[..dim - 1]).is_err());
    let zero = vec![0.0f32; dim];
    assert!(gpu.cosine(&query, &zero).unwrap().is_nan());
}
