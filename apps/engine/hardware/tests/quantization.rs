#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "assertions in tests"
)]
//! Quantized vector encoding and decoding.

use piramid_hardware::compute::quantization::{
    ProductQuantizedVector, QuantizationKind, QuantizedVector,
};

fn int8(vector: &[f32]) -> QuantizedVector {
    QuantizedVector::scalar(vector)
}

#[test]
fn quantization_roundtrip() {
    let original = vec![0.0, 0.5, 1.0, 1.5, 2.0];
    let dequantized = int8(&original).to_f32().unwrap();

    for (o, d) in original.iter().zip(dequantized.iter()) {
        let error = (o - d).abs();
        assert!(error < 0.01, "Error too large: {o} vs {d}");
    }
}

#[test]
fn quantization_constant_vector() {
    let original = vec![1.0, 1.0, 1.0, 1.0];
    let dequantized = int8(&original).to_f32().unwrap();
    for (o, d) in original.iter().zip(dequantized.iter()) {
        assert!((o - d).abs() < 0.001);
    }
}

#[test]
fn quantization_negative_values() {
    let original = vec![-1.0, -0.5, 0.0, 0.5, 1.0];
    let dequantized = int8(&original).to_f32().unwrap();
    for (o, d) in original.iter().zip(dequantized.iter()) {
        let error = (o - d).abs();
        assert!(error < 0.01, "Error too large: {o} vs {d}");
    }
}

#[test]
fn quantization_pq_roundtrip() {
    let original: Vec<f32> = (0..32).map(|i| i as f32 * 0.1).collect();
    let pq = QuantizedVector::pq(&original, 4).unwrap();
    let restored = pq.to_f32().unwrap();
    assert_eq!(restored.len(), original.len());
}

#[test]
fn a_pq_block_count_the_vector_cannot_hold_is_an_error() {
    let vector = [1.0, 2.0, 3.0, 4.0];
    assert!(QuantizedVector::pq(&vector, 0).is_err());
    assert!(QuantizedVector::pq(&vector, 5).is_err());
    assert!(ProductQuantizedVector::from_f32(&vector, 0).is_err());
    let pq = ProductQuantizedVector::from_f32(&vector, 4).unwrap();
    assert_eq!(pq.subquantizers, 4);
}

#[test]
fn corrupt_pq_encoding_fails_decode() {
    let corrupt = QuantizedVector {
        values: Vec::new(),
        min: 0.0,
        max: 0.0,
        pq: Some(ProductQuantizedVector {
            codes: vec![1],
            block_mins: vec![0.0],
            block_maxs: vec![1.0],
            dim: 4,
            subquantizers: 1,
        }),
        kind: QuantizationKind::Pq,
    };

    assert!(corrupt.to_f32().is_err());
}
