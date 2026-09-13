//! Batched distance kernels: one query against a slab of candidates, plus top-k selection.

use crate::gpu::buffer::DeviceBuffer;
use crate::gpu::device::Device;
use crate::gpu::error::{GpuError, GpuResult};
use crate::gpu::module::{KernelArg, KernelModule, LaunchConfig};
use crate::gpu::stream::Stream;

/// Largest k the device selection serves.
pub const MAX_TOP_K: usize = 1024;

/// Scores each selection thread scans.
const SELECT_CHUNK: usize = 8192;

const SOURCE: &str = include_str!("distance.cu");

const FUNCTIONS: [&str; 5] = [
    "cosine_rows",
    "dot_rows",
    "euclidean_rows",
    "euclidean_squared_rows",
    "select_top_k",
];

/// Arguments for one batched distance launch. The buffers are borrowed, not owned.
#[derive(Debug)]
pub struct DistanceLaunch<'a> {
    /// Query vector, dim elements.
    pub query: &'a DeviceBuffer<f32>,
    /// Candidate slab, row-major, rows times dim elements.
    pub candidates: &'a DeviceBuffer<f32>,
    /// Output scores, one per row.
    pub out: &'a mut DeviceBuffer<f32>,
    /// Vector dimensionality.
    pub dim: usize,
    /// Number of candidate rows.
    pub rows: usize,
}

impl DistanceLaunch<'_> {
    fn check(&self) -> GpuResult<(u32, u32)> {
        let mismatch = |what: &str, expected: usize, got: usize| {
            Err(GpuError::Launch(format!(
                "{what} holds {got} elements, expected {expected}"
            )))
        };
        if self.dim == 0 {
            return Err(GpuError::Launch("dim is zero".to_string()));
        }
        if self.query.len() != self.dim {
            return mismatch("query", self.dim, self.query.len());
        }
        let slab = self.rows.saturating_mul(self.dim);
        if self.candidates.len() != slab {
            return mismatch("candidates", slab, self.candidates.len());
        }
        if self.out.len() != self.rows {
            return mismatch("out", self.rows, self.out.len());
        }
        let dim = u32::try_from(self.dim).map_err(|e| GpuError::Launch(e.to_string()))?;
        let rows = u32::try_from(self.rows).map_err(|e| GpuError::Launch(e.to_string()))?;
        Ok((dim, rows))
    }
}

/// The k best rows of a scored batch, highest score first, held on the device.
#[derive(Debug)]
pub struct DeviceTopK {
    /// Scores, k elements, descending. Slots past the row count hold f32::MIN.
    pub scores: DeviceBuffer<f32>,
    /// Row indices matching scores. Slots past the row count hold u32::MAX.
    pub indices: DeviceBuffer<u32>,
}

/// The distance kernel module compiled for one device.
#[derive(Debug)]
pub struct DistanceModule {
    module: KernelModule,
    block_size: u32,
}

impl DistanceModule {
    /// Compile the distance kernels for a device, launching block_size threads per block.
    pub fn compile(device: &Device, block_size: u32) -> GpuResult<Self> {
        if block_size == 0 {
            return Err(GpuError::Launch("block size is zero".to_string()));
        }
        Ok(Self {
            module: KernelModule::compile(device, "distance", SOURCE, &FUNCTIONS)?,
            block_size,
        })
    }

    /// Device the module is compiled for.
    pub fn device(&self) -> &Device {
        self.module.device()
    }

    /// Queue the batched cosine-similarity kernel; scores NaN when a row or the query is zero.
    pub fn cosine_batch(
        &self,
        launch: DistanceLaunch<'_>,
        query_norm_squared: f32,
        stream: &Stream,
    ) -> GpuResult<()> {
        let (dim, rows) = launch.check()?;
        self.module.launch(
            "cosine_rows",
            LaunchConfig::for_elements(launch.rows, self.block_size)?,
            stream,
            &[
                KernelArg::buffer(launch.query),
                KernelArg::buffer(launch.candidates),
                KernelArg::buffer(launch.out),
                KernelArg::U32(dim),
                KernelArg::U32(rows),
                KernelArg::F32(query_norm_squared),
            ],
        )
    }

    /// Queue the batched inner-product kernel.
    pub fn dot_batch(&self, launch: DistanceLaunch<'_>, stream: &Stream) -> GpuResult<()> {
        self.plain("dot_rows", launch, stream)
    }

    /// Queue the batched L2 distance kernel.
    pub fn euclidean_batch(&self, launch: DistanceLaunch<'_>, stream: &Stream) -> GpuResult<()> {
        self.plain("euclidean_rows", launch, stream)
    }

    /// Queue the batched squared L2 distance kernel.
    pub fn euclidean_squared_batch(
        &self,
        launch: DistanceLaunch<'_>,
        stream: &Stream,
    ) -> GpuResult<()> {
        self.plain("euclidean_squared_rows", launch, stream)
    }

    fn plain(
        &self,
        function: &'static str,
        launch: DistanceLaunch<'_>,
        stream: &Stream,
    ) -> GpuResult<()> {
        let (dim, rows) = launch.check()?;
        self.module.launch(
            function,
            LaunchConfig::for_elements(launch.rows, self.block_size)?,
            stream,
            &[
                KernelArg::buffer(launch.query),
                KernelArg::buffer(launch.candidates),
                KernelArg::buffer(launch.out),
                KernelArg::U32(dim),
                KernelArg::U32(rows),
            ],
        )
    }

    /// Select the k highest of the first rows scores, on the device; NaN scores are never picked.
    pub fn top_k(
        &self,
        scores: &DeviceBuffer<f32>,
        rows: usize,
        k: usize,
        stream: &Stream,
    ) -> GpuResult<DeviceTopK> {
        if k == 0 || k > MAX_TOP_K {
            return Err(GpuError::Launch(format!(
                "k must be between 1 and {MAX_TOP_K}, got {k}"
            )));
        }
        if rows == 0 || rows > scores.len() {
            return Err(GpuError::Launch(format!(
                "{rows} rows requested from {} scores",
                scores.len()
            )));
        }
        let chunk = SELECT_CHUNK.max(k * 4);
        let mut pass = self.select_pass(scores, None, rows, chunk, k, stream)?;
        let mut n = pass.scores.len();
        while n > k {
            pass = self.select_pass(&pass.scores, Some(&pass.indices), n, chunk, k, stream)?;
            n = pass.scores.len();
        }
        Ok(pass)
    }

    fn select_pass(
        &self,
        scores: &DeviceBuffer<f32>,
        indices: Option<&DeviceBuffer<u32>>,
        n: usize,
        chunk: usize,
        k: usize,
        stream: &Stream,
    ) -> GpuResult<DeviceTopK> {
        let device = self.module.device();
        let chunks = n.div_ceil(chunk);
        let out = DeviceTopK {
            scores: DeviceBuffer::alloc(device, chunks * k)?,
            indices: DeviceBuffer::alloc(device, chunks * k)?,
        };
        let to_u32 =
            |value: usize| u32::try_from(value).map_err(|e| GpuError::Launch(e.to_string()));
        self.module.launch(
            "select_top_k",
            LaunchConfig::for_elements(chunks, self.block_size)?,
            stream,
            &[
                KernelArg::buffer(scores),
                KernelArg::Pointer(indices.map_or(0, |buffer| buffer.handle().ptr)),
                KernelArg::U32(u32::from(indices.is_some())),
                KernelArg::U32(to_u32(n)?),
                KernelArg::U32(to_u32(chunk)?),
                KernelArg::U32(to_u32(k)?),
                KernelArg::buffer(&out.scores),
                KernelArg::buffer(&out.indices),
            ],
        )?;
        Ok(out)
    }
}
