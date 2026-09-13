//! Reading safetensors checkpoints into named tensors on a device.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use candle_core::{DType, Device, Tensor};
use piramid_core::error::InferenceError;

use crate::inference::backends::candle::runtime::runtime;

/// Named tensors of one checkpoint, taken out one at a time as layers are built.
#[derive(Debug)]
pub struct Weights {
    tensors: HashMap<String, Tensor>,
    dtype: DType,
}

impl Weights {
    /// Load every safetensors shard in a checkpoint directory onto device, converted to dtype.
    pub fn load(dir: &Path, device: &Device, dtype: DType) -> Result<Self, InferenceError> {
        let mut tensors = HashMap::new();
        for shard in shards(dir)? {
            let bytes = std::fs::read(&shard)
                .map_err(|e| InferenceError::Load(format!("{}: {e}", shard.display())))?;
            let loaded = candle_core::safetensors::load_buffer(&bytes, device)
                .map_err(|e| InferenceError::Load(format!("{}: {e}", shard.display())))?;
            tensors.extend(loaded);
        }
        Ok(Self { tensors, dtype })
    }

    /// Wrap tensors already in memory.
    pub fn from_tensors(tensors: HashMap<String, Tensor>, dtype: DType) -> Self {
        Self { tensors, dtype }
    }

    /// Take a tensor by name, checking its shape and converting it to the model dtype.
    pub fn take(&mut self, name: &str, shape: &[usize]) -> Result<Tensor, InferenceError> {
        let tensor = self
            .tensors
            .remove(name)
            .ok_or_else(|| InferenceError::Load(format!("checkpoint has no tensor {name}")))?;
        if tensor.dims() != shape {
            return Err(InferenceError::Load(format!(
                "tensor {name} has shape {:?}, expected {shape:?}",
                tensor.dims()
            )));
        }
        tensor.to_dtype(self.dtype).map_err(runtime)
    }

    /// Whether a tensor by this name is present.
    pub fn contains(&self, name: &str) -> bool {
        self.tensors.contains_key(name)
    }

    /// Names of tensors not taken, sorted.
    pub fn remaining(&self) -> Vec<String> {
        let mut names: Vec<String> = self.tensors.keys().cloned().collect();
        names.sort();
        names
    }
}

fn shards(dir: &Path) -> Result<Vec<PathBuf>, InferenceError> {
    let single = dir.join("model.safetensors");
    if single.is_file() {
        return Ok(vec![single]);
    }
    let index = dir.join("model.safetensors.index.json");
    let text = std::fs::read_to_string(&index).map_err(|e| {
        InferenceError::Load(format!(
            "{} holds neither model.safetensors nor a readable model.safetensors.index.json: {e}",
            dir.display()
        ))
    })?;
    let json: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| InferenceError::Load(format!("{}: {e}", index.display())))?;
    let map = json
        .get("weight_map")
        .and_then(serde_json::Value::as_object)
        .ok_or_else(|| InferenceError::Load(format!("{} has no weight_map", index.display())))?;
    let mut files: Vec<String> = map
        .values()
        .filter_map(|v| v.as_str().map(str::to_string))
        .collect();
    files.sort();
    files.dedup();
    Ok(files.into_iter().map(|file| dir.join(file)).collect())
}
