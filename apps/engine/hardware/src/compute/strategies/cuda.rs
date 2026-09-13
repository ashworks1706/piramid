//! CUDA strategy: every call uploads the query and candidates to the installed device, runs the
//! distance kernels there, and downloads the scores, holding the bytes in the index pool of the
//! device budget meanwhile. A single pair is scored as a batch of one row.

use std::sync::OnceLock;

use crate::compute::error::{ComputeError, ComputeResult};
use crate::compute::kernels::{check_batch_shape, DistanceKernels};
use crate::compute::mode::ExecutionMode;
use crate::gpu::kernels::distance::{DistanceLaunch, DistanceModule};
use crate::gpu::{DeviceBudget, DeviceBuffer, GpuError, GpuManager, MemoryPool, Stream};

/// Device kernels on the first CUDA device.
#[derive(Debug, Default, Clone, Copy)]
pub struct CudaStrategy;

struct State {
    module: DistanceModule,
    stream: Stream,
    budget: DeviceBudget,
}

static STATE: OnceLock<State> = OnceLock::new();

/// Serve the gpu mode from a manager's device, on its first stream, compiling the kernels at
/// block_size threads per block. A process installs one device.
pub fn install_gpu(manager: &GpuManager, block_size: u32) -> ComputeResult<()> {
    let stream = manager.streams().first().cloned().ok_or_else(|| {
        failed(GpuError::Runtime(
            "the GPU manager opened no stream".to_string(),
        ))
    })?;
    let module = DistanceModule::compile(manager.device(), block_size).map_err(failed)?;
    STATE
        .set(State {
            module,
            stream,
            budget: manager.budget().clone(),
        })
        .map_err(|_| ComputeError::StrategyFailed {
            strategy: "cuda",
            message: "a GPU is already installed for this process".to_string(),
        })
}

fn state() -> ComputeResult<&'static State> {
    STATE
        .get()
        .ok_or_else(|| ComputeError::StrategyUnavailable {
            strategy: "cuda",
            reason: "no GPU is installed; startup.hardware.profile gpu opens one at startup"
                .to_string(),
        })
}

fn failed(error: GpuError) -> ComputeError {
    ComputeError::StrategyFailed {
        strategy: "cuda",
        message: error.to_string(),
    }
}

#[derive(Clone, Copy)]
enum Kind {
    Cosine,
    Dot,
    Euclidean,
}

fn run(
    kind: Kind,
    query: &[f32],
    candidates: &[f32],
    dim: usize,
    out: &mut [f32],
) -> ComputeResult<()> {
    let rows = check_batch_shape(query, candidates, dim, out)?;
    if rows == 0 {
        return Ok(());
    }
    let state = state()?;
    let bytes = ((query.len() + candidates.len() + rows) * std::mem::size_of::<f32>()) as u64;
    let _held = state
        .budget
        .reserve(MemoryPool::Index, bytes)
        .map_err(failed)?;
    let device = state.module.device();
    let stream = &state.stream;
    let query_gpu = DeviceBuffer::from_host(device, query, stream).map_err(failed)?;
    let slab_gpu = DeviceBuffer::from_host(device, candidates, stream).map_err(failed)?;
    let mut out_gpu = DeviceBuffer::<f32>::alloc(device, rows).map_err(failed)?;
    let launch = DistanceLaunch {
        query: &query_gpu,
        candidates: &slab_gpu,
        out: &mut out_gpu,
        dim,
        rows,
    };
    match kind {
        Kind::Cosine => {
            let norm = query.iter().map(|x| x * x).sum();
            state.module.cosine_batch(launch, norm, stream)
        }
        Kind::Dot => state.module.dot_batch(launch, stream),
        Kind::Euclidean => state.module.euclidean_batch(launch, stream),
    }
    .map_err(failed)?;
    out_gpu.copy_to_host(out, stream).map_err(failed)
}

fn pair(kind: Kind, a: &[f32], b: &[f32]) -> f32 {
    let mut out = [0.0f32];
    match run(kind, a, b, a.len(), &mut out) {
        Ok(()) => out[0],
        Err(error) => {
            tracing::error!(target: "piramid::compute", %error, "cuda pairwise score failed");
            f32::NAN
        }
    }
}

impl DistanceKernels for CudaStrategy {
    fn mode(&self) -> ExecutionMode {
        ExecutionMode::Gpu
    }

    fn name(&self) -> &'static str {
        "cuda"
    }

    fn is_available(&self) -> bool {
        state().is_ok()
    }

    fn cosine(&self, a: &[f32], b: &[f32]) -> f32 {
        pair(Kind::Cosine, a, b)
    }

    fn dot(&self, a: &[f32], b: &[f32]) -> f32 {
        pair(Kind::Dot, a, b)
    }

    fn euclidean(&self, a: &[f32], b: &[f32]) -> f32 {
        pair(Kind::Euclidean, a, b)
    }

    fn euclidean_squared(&self, a: &[f32], b: &[f32]) -> f32 {
        let distance = pair(Kind::Euclidean, a, b);
        distance * distance
    }

    fn cosine_batch(
        &self,
        query: &[f32],
        candidates: &[f32],
        dim: usize,
        out: &mut [f32],
    ) -> ComputeResult<()> {
        run(Kind::Cosine, query, candidates, dim, out)
    }

    fn dot_batch(
        &self,
        query: &[f32],
        candidates: &[f32],
        dim: usize,
        out: &mut [f32],
    ) -> ComputeResult<()> {
        run(Kind::Dot, query, candidates, dim, out)
    }

    fn euclidean_batch(
        &self,
        query: &[f32],
        candidates: &[f32],
        dim: usize,
        out: &mut [f32],
    ) -> ComputeResult<()> {
        run(Kind::Euclidean, query, candidates, dim, out)
    }
}
