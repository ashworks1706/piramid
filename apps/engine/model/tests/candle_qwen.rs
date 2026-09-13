//! The Qwen decoders on candle: rotary embedding layout, cache slot runs, cache checks and a
//! real checkpoint against the transformers reference.
#![cfg(feature = "inference-candle")]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "assertions in tests"
)]

use std::path::PathBuf;

use candle_core::{DType, Device, Tensor};
use piramid_model::inference::architecture::{
    Architecture, DecoderModel, ModelSpec, Precision, StepBatch, StepSequence,
};
use piramid_model::inference::backends::candle::qwen::testing::{drop_cache_storage, tiny_model};
use piramid_model::inference::backends::candle::qwen::{runs, QwenModel, WriteRun};
use piramid_model::inference::backends::candle::weights::Weights;

#[test]
fn rotating_keys_tokens_first_matches_rotating_them_heads_first() {
    let (heads, tokens, dim) = (3, 5, 8);
    let values: Vec<f32> = (0..heads * tokens * dim)
        .map(|i| (i as f32 * 0.37).sin())
        .collect();
    let heads_first = Tensor::from_vec(values, (1, heads, tokens, dim), &Device::Cpu).unwrap();
    let angles: Vec<f32> = (0..tokens * dim / 2).map(|i| i as f32 * 0.11).collect();
    let angles = Tensor::from_vec(angles, (tokens, dim / 2), &Device::Cpu).unwrap();
    let (cos, sin) = (angles.cos().unwrap(), angles.sin().unwrap());

    let expected = candle_nn::rotary_emb::rope(&heads_first, &cos, &sin).unwrap();
    let tokens_first = heads_first.transpose(1, 2).unwrap().contiguous().unwrap();
    let got = candle_nn::rotary_emb::rope_thd(&tokens_first, &cos, &sin)
        .unwrap()
        .transpose(1, 2)
        .unwrap();
    let difference = (expected - got)
        .unwrap()
        .abs()
        .unwrap()
        .max_all()
        .unwrap()
        .to_scalar::<f32>()
        .unwrap();
    assert!(difference < 1e-6, "{difference}");
}

#[test]
fn slots_group_into_contiguous_runs() {
    let run = |slot, token, len| WriteRun { slot, token, len };
    assert_eq!(
        runs(&[4, 5, 6, 12, 13, 0]),
        vec![run(4, 0, 3), run(12, 3, 2), run(0, 5, 1)]
    );
    assert!(runs(&[]).is_empty());
}

#[test]
fn a_layer_without_cache_storage_is_an_error() {
    let mut model = tiny_model(Architecture::Qwen2, 3, 8);
    let batch = StepBatch {
        sequences: vec![StepSequence {
            tokens: vec![1, 2],
            start: 0,
            write_slots: vec![0, 1],
            context_slots: vec![0, 1],
            logits: true,
        }],
    };
    let mut pass = model.begin(&batch).unwrap();
    drop_cache_storage(&mut model);
    assert!(model.layer(&mut pass, 0).is_err());
}

#[test]
fn a_write_slot_outside_the_cache_is_refused() {
    let mut model = tiny_model(Architecture::Qwen2, 3, 8);
    let batch = StepBatch {
        sequences: vec![StepSequence {
            tokens: vec![1],
            start: 1,
            write_slots: vec![8],
            context_slots: vec![0, 1],
            logits: true,
        }],
    };
    assert!(model.begin(&batch).is_err());
}

#[test]
#[ignore = "needs PIRAMID_TEST_MODEL pointing at Qwen2.5-0.5B-Instruct"]
fn the_checkpoint_matches_the_transformers_reference() {
    let dir = PathBuf::from(std::env::var("PIRAMID_TEST_MODEL").expect("PIRAMID_TEST_MODEL"));
    let fixture: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/qwen2.5-0.5b-instruct.json"
        ))
        .unwrap(),
    )
    .unwrap();
    let ids = |key: &str| -> Vec<u32> {
        fixture[key]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_u64().unwrap() as u32)
            .collect()
    };
    let prompt = ids("prompt_ids");
    let spec = ModelSpec::from_dir(&dir).unwrap();
    let device = Device::Cpu;
    let weights = Weights::load(&dir, &device, DType::F32).unwrap();
    let mut model =
        QwenModel::load(spec, weights, &device, Precision::F32, Precision::F32).unwrap();
    model.allocate_cache(256).unwrap();
    let mut tokens = prompt.clone();
    let mut generated = Vec::new();
    let mut start = 0;
    for step in 0..8 {
        let end = tokens.len();
        let batch = StepBatch {
            sequences: vec![StepSequence {
                tokens: tokens[start..end].to_vec(),
                start,
                write_slots: (start..end).map(|p| p as u32).collect(),
                context_slots: (0..end).map(|p| p as u32).collect(),
                logits: true,
            }],
        };
        let mut pass = model.begin(&batch).unwrap();
        for layer in 0..model.spec().layers {
            model.layer(&mut pass, layer).unwrap();
        }
        let logits = model.finish(pass).unwrap().remove(0);
        if step == 0 {
            let expected = fixture["top5_logits"].as_array().unwrap();
            for (id, value) in ids("top5_ids").iter().zip(expected) {
                let got = logits[*id as usize];
                assert!(
                    (got - value.as_f64().unwrap() as f32).abs() < 2e-3,
                    "token {id}: {got}"
                );
            }
        }
        let next = logits
            .iter()
            .enumerate()
            .fold(0, |best, (i, &v)| if v > logits[best] { i } else { best })
            as u32;
        generated.push(next);
        start = end;
        tokens.push(next);
    }
    assert_eq!(generated, ids("greedy"));
}

#[cfg(feature = "gpu-cuda")]
#[test]
#[ignore = "needs a CUDA device"]
fn a_device_kernel_changes_hidden_state_in_place_on_the_model_stream() {
    use piramid_hardware::gpu::{KernelArg, KernelModule, LaunchConfig};
    use piramid_model::fusion::HiddenState;
    use piramid_model::inference::backends::candle::qwen::testing::tiny_model_on;

    const SOURCE: &str = r#"
extern "C" __global__ void add_constant(float* rows, unsigned int n, float value) {
    unsigned int i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) {
        rows[i] += value;
    }
}
"#;
    let tokens = [3u32, 1, 4, 1, 5];
    let batch = StepBatch {
        sequences: vec![StepSequence {
            tokens: tokens.to_vec(),
            start: 0,
            write_slots: (0..5).collect(),
            context_slots: (0..5).collect(),
            logits: true,
        }],
    };
    let run = |device: &Device| -> (Vec<f32>, &'static str) {
        let mut model = tiny_model_on(Architecture::Qwen3, 5, 16, device);
        let mut pass = model.begin(&batch).unwrap();
        let mut path = "none";
        model
            .with_hidden(&mut pass, 0, &mut |hidden, stream| {
                match hidden {
                    HiddenState::Host(rows) => {
                        path = "host";
                        for value in rows.iter_mut() {
                            *value += 0.5;
                        }
                    }
                    HiddenState::Device(buffer) => {
                        path = "device";
                        let stream = stream.unwrap();
                        let module = KernelModule::compile(
                            buffer.device(),
                            "add_constant",
                            SOURCE,
                            &["add_constant"],
                        )
                        .unwrap();
                        let n = buffer.len();
                        module
                            .launch(
                                "add_constant",
                                LaunchConfig::for_elements(n, 256).unwrap(),
                                stream,
                                &[
                                    KernelArg::buffer(buffer),
                                    KernelArg::U32(n as u32),
                                    KernelArg::F32(0.5),
                                ],
                            )
                            .unwrap();
                    }
                }
                Ok(())
            })
            .unwrap();
        for layer in 0..model.spec().layers {
            model.layer(&mut pass, layer).unwrap();
        }
        (model.finish(pass).unwrap().remove(0), path)
    };
    let (host, host_path) = run(&Device::Cpu);
    let (device, device_path) = run(&Device::new_cuda(0).unwrap());
    assert_eq!((host_path, device_path), ("host", "device"));
    for (a, b) in host.iter().zip(&device) {
        assert!((a - b).abs() < 1e-3, "{a} {b}");
    }
}
