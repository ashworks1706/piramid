//! The precision weights and the key/value cache load at.
#![cfg(feature = "inference-candle")]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "assertions in tests"
)]

use piramid_core::config::Dtype;
use piramid_model::inference::architecture::Precision;
use piramid_model::inference::backends::candle::loader::{cache_precision, weight_precision};

#[test]
fn auto_precision_follows_the_checkpoint_only_on_a_device() {
    assert_eq!(
        weight_precision(Dtype::Auto, Precision::Bf16, true),
        Precision::Bf16
    );
    assert_eq!(
        weight_precision(Dtype::Auto, Precision::Bf16, false),
        Precision::F32
    );
    assert_eq!(
        weight_precision(Dtype::Fp16, Precision::Bf16, false),
        Precision::F16
    );
    assert_eq!(cache_precision(Dtype::Auto, Precision::F16), Precision::F16);
    assert_eq!(cache_precision(Dtype::Fp32, Precision::F16), Precision::F32);
}
