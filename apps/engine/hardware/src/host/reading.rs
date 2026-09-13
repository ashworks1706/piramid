//! One reading of host processor and memory use.

/// Processor and memory use of the host and this process at one instant; None when unmeasurable.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct HostReading {
    /// Processor use across every logical CPU of the host, from 0 to 100.
    pub cpu_percent: Option<f32>,
    /// Physical memory in use on the host, in bytes.
    pub memory_used_bytes: Option<u64>,
    /// Physical memory installed on the host, in bytes.
    pub memory_total_bytes: Option<u64>,
    /// Processor use of this process as a share of every logical CPU of the host, from 0 to 100.
    pub process_cpu_percent: Option<f32>,
    /// Resident memory of this process, in bytes.
    pub process_resident_bytes: Option<u64>,
}

/// Memory, utilisation and temperature of one GPU at one instant; None when the driver can't tell.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct GpuReading {
    /// Index of the device as the driver enumerates it.
    pub index: u32,
    /// Product name the driver reports for the device.
    pub name: Option<String>,
    /// Device memory in use, in bytes.
    pub memory_used_bytes: Option<u64>,
    /// Device memory installed, in bytes.
    pub memory_total_bytes: Option<u64>,
    /// Share of the last sample period during which a kernel ran on the device, from 0 to 100.
    pub utilization_percent: Option<f32>,
    /// Temperature of the device die, in degrees Celsius.
    pub temperature_celsius: Option<f32>,
}
