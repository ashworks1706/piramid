//! Loading a checkpoint directory onto a device at the configured precision.

use std::path::Path;

use piramid_core::config::Dtype;
use piramid_core::error::InferenceError;

use crate::inference::architecture::{ModelSpec, Precision};
use crate::inference::backends::candle::qwen::QwenModel;
use crate::inference::backends::candle::runtime::{dtype, CandleRuntime, DeviceSelection};
use crate::inference::backends::candle::weights::Weights;

/// A decoder loaded onto its device.
#[derive(Debug)]
pub struct LoadedDecoder {
    /// The model, with no cache storage allocated.
    pub model: QwenModel,
    /// The device it runs on.
    pub runtime: CandleRuntime,
}

/// The precision weights load at: auto is the checkpoint precision on a GPU and fp32 on the cpu.
pub fn weight_precision(setting: Dtype, stored: Precision, on_device: bool) -> Precision {
    match setting {
        Dtype::Auto if on_device => stored,
        Dtype::Auto | Dtype::Fp32 => Precision::F32,
        Dtype::Fp16 => Precision::F16,
        Dtype::Bf16 => Precision::Bf16,
    }
}

/// The precision the key/value cache holds: auto matches the weights.
pub fn cache_precision(setting: Dtype, weights: Precision) -> Precision {
    match setting {
        Dtype::Auto => weights,
        Dtype::Fp32 => Precision::F32,
        Dtype::Fp16 => Precision::F16,
        Dtype::Bf16 => Precision::Bf16,
    }
}

/// Open the device named cpu or cuda:N and load the checkpoint in dir onto it.
pub fn load_decoder(
    dir: &Path,
    spec: ModelSpec,
    device: &str,
    weights: Dtype,
    cache: Dtype,
) -> Result<LoadedDecoder, InferenceError> {
    let runtime = CandleRuntime::open(DeviceSelection::parse(device)?)?;
    let precision = weight_precision(weights, spec.stored_precision, runtime.ordinal().is_some());
    let kv_precision = cache_precision(cache, precision);
    let tensors = Weights::load(dir, runtime.device(), dtype(precision))?;
    let model = QwenModel::load(spec, tensors, runtime.device(), precision, kv_precision)?;
    runtime.synchronize()?;
    Ok(LoadedDecoder { model, runtime })
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
