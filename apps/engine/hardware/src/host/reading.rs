//! One reading of host processor and memory use.

/// Processor and memory use of the host and of this process at one instant.
///
/// A field is None when this platform or this sample could not measure it.
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
