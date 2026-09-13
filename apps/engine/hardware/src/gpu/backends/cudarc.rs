//! NVIDIA CUDA backend, built on cudarc; compiled only under the gpu-cuda feature. cudarc types
//! never leave this file.

use std::collections::HashMap;
use std::sync::Arc;

use cudarc::driver::{result, CudaContext, CudaFunction, CudaStream, PushKernelArg};
use cudarc::nvrtc::{compile_ptx_with_opts, CompileOptions};
use parking_lot::RwLock;

use crate::gpu::buffer::DeviceAllocation;
use crate::gpu::device::{Device, DeviceCapabilities, DeviceRuntime};
use crate::gpu::error::{GpuError, GpuResult};
use crate::gpu::module::{KernelArg, LaunchConfig};

/// CUDA device runtime over the primary context of one device.
#[derive(Debug)]
pub struct CudaRuntime {
    capabilities: DeviceCapabilities,
    context: Arc<CudaContext>,
    streams: RwLock<Vec<Arc<CudaStream>>>,
    modules: RwLock<Vec<HashMap<&'static str, CudaFunction>>>,
}

/// Open the device at an ordinal, retaining its primary context.
pub fn open(ordinal: usize) -> GpuResult<Device> {
    let context = CudaContext::new(ordinal).map_err(|e| unavailable(ordinal, &e))?;
    let name = context.name().map_err(runtime)?;
    let (major, minor) = context.compute_capability().map_err(runtime)?;
    let total = context.total_mem().map_err(runtime)?;
    let capabilities = DeviceCapabilities {
        name,
        ordinal,
        compute_capability: (
            u32::try_from(major).map_err(runtime)?,
            u32::try_from(minor).map_err(runtime)?,
        ),
        total_memory_bytes: total as u64,
    };
    let streams = vec![context.default_stream(), context.per_thread_stream()];
    Ok(Device::new(Arc::new(CudaRuntime {
        capabilities,
        context,
        streams: RwLock::new(streams),
        modules: RwLock::new(Vec::new()),
    })))
}

fn unavailable(ordinal: usize, error: &dyn std::fmt::Display) -> GpuError {
    GpuError::Unavailable(format!(
        "CUDA device {ordinal} could not be opened: {error}"
    ))
}

fn runtime(error: impl std::fmt::Display) -> GpuError {
    GpuError::Runtime(error.to_string())
}

impl CudaRuntime {
    fn stream(&self, id: u64) -> GpuResult<Arc<CudaStream>> {
        usize::try_from(id)
            .ok()
            .and_then(|index| self.streams.read().get(index).cloned())
            .ok_or_else(|| GpuError::Runtime(format!("no stream with id {id}")))
    }

    fn bind(&self) -> GpuResult<()> {
        self.context.bind_to_thread().map_err(runtime)
    }
}

impl DeviceRuntime for CudaRuntime {
    fn name(&self) -> &'static str {
        "cudarc"
    }

    fn capabilities(&self) -> &DeviceCapabilities {
        &self.capabilities
    }

    fn available_memory_bytes(&self) -> GpuResult<u64> {
        let (free, _total) = self.context.mem_get_info().map_err(runtime)?;
        Ok(free as u64)
    }

    fn synchronize(&self) -> GpuResult<()> {
        self.context.synchronize().map_err(runtime)
    }

    #[allow(unsafe_code)]
    fn allocate(&self, size_bytes: usize) -> GpuResult<DeviceAllocation> {
        self.bind()?;
        if size_bytes == 0 {
            return Ok(DeviceAllocation { ptr: 0, size_bytes });
        }
        // SAFETY: the context is bound to this thread, and the returned address is owned by the
        // DeviceAllocation until free releases it.
        let ptr = unsafe { result::malloc_sync(size_bytes) }
            .map_err(|e| GpuError::Allocation(format!("{size_bytes} bytes: {e}")))?;
        Ok(DeviceAllocation { ptr, size_bytes })
    }

    #[allow(unsafe_code)]
    fn free(&self, allocation: &DeviceAllocation) -> GpuResult<()> {
        if allocation.ptr == 0 {
            return Ok(());
        }
        self.bind()?;
        self.synchronize()?;
        // SAFETY: the address came from allocate on this context, is freed once by the owning
        // buffer, and every stream has finished using it after the synchronize above.
        unsafe { result::free_sync(allocation.ptr) }.map_err(runtime)
    }

    #[allow(unsafe_code)]
    fn copy_to_device(&self, dst: &DeviceAllocation, src: &[u8], stream: u64) -> GpuResult<()> {
        if src.len() > dst.size_bytes {
            return Err(GpuError::Transfer(format!(
                "{} host bytes do not fit a {} byte allocation",
                src.len(),
                dst.size_bytes
            )));
        }
        if src.is_empty() {
            return Ok(());
        }
        let stream = self.stream(stream)?;
        self.bind()?;
        // SAFETY: dst is a live device region of at least src.len() bytes on this context, and
        // src stays borrowed until the stream synchronize below has completed the copy.
        unsafe { result::memcpy_htod_async(dst.ptr, src, stream.cu_stream()) }
            .map_err(|e| GpuError::Transfer(e.to_string()))?;
        stream.synchronize().map_err(runtime)
    }

    #[allow(unsafe_code)]
    fn copy_to_host(&self, src: &DeviceAllocation, dst: &mut [u8], stream: u64) -> GpuResult<()> {
        if dst.len() > src.size_bytes {
            return Err(GpuError::Transfer(format!(
                "{} host bytes exceed a {} byte allocation",
                dst.len(),
                src.size_bytes
            )));
        }
        if dst.is_empty() {
            return Ok(());
        }
        let stream = self.stream(stream)?;
        self.bind()?;
        // SAFETY: src is a live device region of at least dst.len() bytes on this context, and
        // dst stays exclusively borrowed until the stream synchronize below has completed the copy.
        unsafe { result::memcpy_dtoh_async(dst, src.ptr, stream.cu_stream()) }
            .map_err(|e| GpuError::Transfer(e.to_string()))?;
        stream.synchronize().map_err(runtime)
    }

    fn create_stream(&self) -> GpuResult<u64> {
        let stream = self.context.new_stream().map_err(runtime)?;
        let mut streams = self.streams.write();
        streams.push(stream);
        Ok((streams.len() - 1) as u64)
    }

    fn synchronize_stream(&self, stream: u64) -> GpuResult<()> {
        let stream = self.stream(stream)?;
        self.bind()?;
        stream.synchronize().map_err(runtime)
    }

    fn compile_module(&self, source: &str, functions: &[&'static str]) -> GpuResult<u64> {
        let options = CompileOptions {
            arch: None,
            ..Default::default()
        };
        let ptx = compile_ptx_with_opts(source, options)
            .map_err(|e| GpuError::ModuleLoad(format!("nvrtc: {e}")))?;
        self.bind()?;
        let module = self
            .context
            .load_module(ptx)
            .map_err(|e| GpuError::ModuleLoad(e.to_string()))?;
        let mut loaded = HashMap::with_capacity(functions.len());
        for &function in functions {
            let handle = module
                .load_function(function)
                .map_err(|e| GpuError::ModuleLoad(format!("{function}: {e}")))?;
            loaded.insert(function, handle);
        }
        let mut modules = self.modules.write();
        modules.push(loaded);
        Ok((modules.len() - 1) as u64)
    }

    #[allow(unsafe_code)]
    fn launch(
        &self,
        module: u64,
        function: &'static str,
        config: LaunchConfig,
        stream: u64,
        args: &[KernelArg],
    ) -> GpuResult<()> {
        let stream = self.stream(stream)?;
        let modules = self.modules.read();
        let func = usize::try_from(module)
            .ok()
            .and_then(|index| modules.get(index))
            .and_then(|functions| functions.get(function))
            .ok_or_else(|| {
                GpuError::Launch(format!("no function {function} in module {module}"))
            })?;

        self.bind()?;
        let mut builder = stream.launch_builder(func);
        for arg in args {
            match arg {
                KernelArg::Pointer(value) | KernelArg::U64(value) => builder.arg(value),
                KernelArg::U32(value) => builder.arg(value),
                KernelArg::I32(value) => builder.arg(value),
                KernelArg::F32(value) => builder.arg(value),
            };
        }
        let config = cudarc::driver::LaunchConfig {
            grid_dim: config.grid,
            block_dim: config.block,
            shared_mem_bytes: config.shared_memory_bytes,
        };
        // SAFETY: func was loaded from a module on this context, every argument is borrowed from
        // args for the whole call in the order the kernel declares, and every pointer argument
        // names device memory on this context that the caller keeps alive until the stream has
        // executed the launch.
        unsafe { builder.launch(config) }
            .map(|_| ())
            .map_err(|e| GpuError::Launch(format!("{function}: {e}")))
    }
}
