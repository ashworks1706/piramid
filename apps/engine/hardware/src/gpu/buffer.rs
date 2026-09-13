//! Device memory: [DeviceBuffer] holds vectors and weights that are uploaded once and reused.

use std::marker::PhantomData;

use crate::gpu::device::Device;
use crate::gpu::error::{GpuError, GpuResult};
use crate::gpu::stream::Stream;

/// A typed region of device memory, generic over element type (f32, f16, u32 and so on).
///
/// An owned buffer frees its allocation on drop. A borrowed buffer names memory another runtime
/// on the same device owns, and never frees it.
#[derive(Debug)]
pub struct DeviceBuffer<T> {
    device: Device,
    handle: DeviceAllocation,
    len: usize,
    owned: bool,
    _marker: PhantomData<T>,
}

/// Address and size of a region of device memory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeviceAllocation {
    /// Device address, valid in the context of the device that holds it.
    pub ptr: u64,
    /// Region size in bytes.
    pub size_bytes: usize,
}

impl<T: Copy> DeviceBuffer<T> {
    /// Allocate the given number of elements on a device. The contents are unspecified.
    pub fn alloc(device: &Device, len: usize) -> GpuResult<Self> {
        let size_bytes = len
            .checked_mul(std::mem::size_of::<T>())
            .ok_or_else(|| GpuError::Allocation(format!("{len} elements overflow usize")))?;
        let handle = device.runtime().allocate(size_bytes)?;
        Ok(Self {
            device: device.clone(),
            handle,
            len,
            owned: true,
            _marker: PhantomData,
        })
    }

    /// Allocate and fill from a host slice in one step.
    pub fn from_host(device: &Device, src: &[T], stream: &Stream) -> GpuResult<Self> {
        let mut buffer = Self::alloc(device, src.len())?;
        buffer.copy_from_host(src, stream)?;
        Ok(buffer)
    }

    /// Name len elements of device memory at ptr that another runtime on this device owns.
    ///
    /// The caller keeps that memory alive and unaliased for as long as the returned buffer is
    /// used; the buffer never frees it.
    pub fn borrowed(device: &Device, ptr: u64, len: usize) -> GpuResult<Self> {
        let size_bytes = len
            .checked_mul(std::mem::size_of::<T>())
            .ok_or_else(|| GpuError::Allocation(format!("{len} elements overflow usize")))?;
        Ok(Self {
            device: device.clone(),
            handle: DeviceAllocation { ptr, size_bytes },
            len,
            owned: false,
            _marker: PhantomData,
        })
    }

    /// Copy a host slice of exactly len elements into this buffer.
    pub fn copy_from_host(&mut self, src: &[T], stream: &Stream) -> GpuResult<()> {
        self.check_len(src.len())?;
        self.device
            .runtime()
            .copy_to_device(&self.handle, as_bytes(src), stream.id())
    }

    /// Copy the contents of this buffer into a host slice of exactly len elements.
    pub fn copy_to_host(&self, dst: &mut [T], stream: &Stream) -> GpuResult<()> {
        self.check_len(dst.len())?;
        self.device
            .runtime()
            .copy_to_host(&self.handle, as_bytes_mut(dst), stream.id())
    }

    /// Copy the contents of this buffer into a new host vector.
    pub fn to_host(&self, stream: &Stream) -> GpuResult<Vec<T>>
    where
        T: Default,
    {
        let mut out = vec![T::default(); self.len];
        self.copy_to_host(&mut out, stream)?;
        Ok(out)
    }

    fn check_len(&self, host_len: usize) -> GpuResult<()> {
        if host_len == self.len {
            Ok(())
        } else {
            Err(GpuError::Transfer(format!(
                "host slice has {host_len} elements, device buffer has {}",
                self.len
            )))
        }
    }
}

impl<T> DeviceBuffer<T> {
    /// Number of elements.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Whether the buffer holds no elements.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Device this buffer lives on.
    pub fn device(&self) -> &Device {
        &self.device
    }

    /// Address and size, for kernel argument binding.
    pub fn handle(&self) -> &DeviceAllocation {
        &self.handle
    }
}

impl<T> Drop for DeviceBuffer<T> {
    fn drop(&mut self) {
        if self.owned {
            if let Err(error) = self.device.runtime().free(&self.handle) {
                tracing::warn!(target: "piramid::gpu", %error, "device free failed");
            }
        }
    }
}

/// Reinterpret a typed slice as bytes for transfer.
#[allow(unsafe_code)]
fn as_bytes<T: Copy>(src: &[T]) -> &[u8] {
    // SAFETY: T is Copy with no drop glue, and the returned slice borrows src for its lifetime
    // with a length of size_of_val(src).
    unsafe { std::slice::from_raw_parts(src.as_ptr().cast::<u8>(), std::mem::size_of_val(src)) }
}

/// Reinterpret a typed slice as mutable bytes for transfer.
#[allow(unsafe_code)]
fn as_bytes_mut<T: Copy>(dst: &mut [T]) -> &mut [u8] {
    let size = std::mem::size_of_val(dst);
    // SAFETY: T is Copy plain numeric data for which any byte pattern is valid, and the returned
    // slice holds the exclusive borrow of dst with a length of size_of_val(dst).
    unsafe { std::slice::from_raw_parts_mut(dst.as_mut_ptr().cast::<u8>(), size) }
}
