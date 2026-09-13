//! GPU readings from the NVIDIA Management Library, loaded from the driver at runtime.

use nvml_wrapper::enum_wrappers::device::TemperatureSensor;
use nvml_wrapper::{Device, Nvml};

use crate::host::reading::GpuReading;

/// A loaded NVIDIA Management Library.
#[derive(Debug)]
pub(crate) struct Library {
    nvml: Nvml,
}

impl Library {
    /// Loads the library, or returns the reason it did not load.
    pub(crate) fn load() -> Result<Self, String> {
        Nvml::init()
            .map(|nvml| Self { nvml })
            .map_err(|error| error.to_string())
    }

    /// One reading per device the library can open, or the reason the devices could not be
    /// counted.
    pub(crate) fn sample(&self) -> Result<Vec<GpuReading>, String> {
        let count = self
            .nvml
            .device_count()
            .map_err(|error| error.to_string())?;
        Ok((0..count)
            .filter_map(|index| {
                let device = self.nvml.device_by_index(index).ok()?;
                Some(read(index, &device))
            })
            .collect())
    }
}

/// Reads one device, leaving out each field the driver does not report.
fn read(index: u32, device: &Device<'_>) -> GpuReading {
    let memory = device.memory_info().ok();
    GpuReading {
        index,
        name: device.name().ok(),
        memory_used_bytes: memory.as_ref().map(|memory| memory.used),
        memory_total_bytes: memory
            .as_ref()
            .map(|memory| memory.total)
            .filter(|total| *total > 0),
        utilization_percent: device
            .utilization_rates()
            .ok()
            .map(|rates| rates.gpu as f32),
        temperature_celsius: device
            .temperature(TemperatureSensor::Gpu)
            .ok()
            .map(|celsius| celsius as f32),
    }
}
