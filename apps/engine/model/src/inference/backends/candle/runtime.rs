//! Device selection and precision mapping for candle.

use candle_core::{DType, Device};
use piramid_core::config::DeviceSelection;
use piramid_core::error::InferenceError;

use crate::inference::architecture::Precision;

/// Where a model runs, as candle names it.
#[derive(Debug, Clone)]
pub struct CandleRuntime {
    device: Device,
    ordinal: Option<usize>,
}

impl CandleRuntime {
    /// Open the selected device.
    pub fn open(selection: DeviceSelection) -> Result<Self, InferenceError> {
        match selection {
            DeviceSelection::Cpu => Ok(Self {
                device: Device::Cpu,
                ordinal: None,
            }),
            DeviceSelection::Cuda(ordinal) => {
                if !candle_core::utils::cuda_is_available() {
                    return Err(InferenceError::Unavailable(format!(
                        "cuda:{ordinal} requested but this build has no CUDA model runtime; rebuild with the gpu-cuda feature"
                    )));
                }
                let device = Device::new_cuda(ordinal).map_err(|e| {
                    InferenceError::Unavailable(format!("cuda:{ordinal} could not be opened: {e}"))
                })?;
                Ok(Self {
                    device,
                    ordinal: Some(ordinal),
                })
            }
        }
    }

    /// Runtime name, for logs and configuration.
    pub fn name(&self) -> &'static str {
        "candle"
    }

    /// The candle device.
    pub fn device(&self) -> &Device {
        &self.device
    }

    /// CUDA ordinal, or None on the CPU.
    pub fn ordinal(&self) -> Option<usize> {
        self.ordinal
    }

    /// Block until queued device work completes.
    pub fn synchronize(&self) -> Result<(), InferenceError> {
        self.device.synchronize().map_err(runtime)
    }
}

/// The candle dtype of a precision.
pub fn dtype(precision: Precision) -> DType {
    match precision {
        Precision::F32 => DType::F32,
        Precision::F16 => DType::F16,
        Precision::Bf16 => DType::BF16,
    }
}

/// A candle failure as an inference runtime error.
pub fn runtime(error: candle_core::Error) -> InferenceError {
    InferenceError::Runtime(error.to_string())
}
