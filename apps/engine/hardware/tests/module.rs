//! Launch geometry of compiled kernel modules.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "assertions in tests"
)]

use piramid_hardware::gpu::{GpuError, LaunchConfig};

#[test]
fn a_launch_covers_every_element() {
    let config = LaunchConfig::for_elements(1000, 256).unwrap();
    assert_eq!(config.grid, (4, 1, 1));
    assert_eq!(config.block, (256, 1, 1));
}

#[test]
fn a_zero_block_size_is_refused() {
    assert!(matches!(
        LaunchConfig::for_elements(10, 0),
        Err(GpuError::Launch(_))
    ));
}

#[test]
fn a_block_count_beyond_the_grid_is_refused() {
    let elements = (u32::MAX as usize) * 2;
    assert!(matches!(
        LaunchConfig::for_elements(elements, 1),
        Err(GpuError::Launch(_))
    ));
}
