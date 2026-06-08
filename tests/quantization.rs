use piramid::quantization::{ProductQuantizedVector, QuantizationKind, QuantizedVector};

#[test]
fn quantization_roundtrip() {
    let original = vec![0.0, 0.5, 1.0, 1.5, 2.0];
    let quantized = QuantizedVector::from_f32(&original);
    let dequantized = quantized.to_f32();

    for (o, d) in original.iter().zip(dequantized.iter()) {
        let error = (o - d).abs();
        assert!(error < 0.01, "Error too large: {} vs {}", o, d);
    }
}

#[test]
fn quantization_constant_vector() {
    let original = vec![1.0, 1.0, 1.0, 1.0];
    let quantized = QuantizedVector::from_f32(&original);
    let dequantized = quantized.to_f32();
    for (o, d) in original.iter().zip(dequantized.iter()) {
        assert!((o - d).abs() < 0.001);
    }
}

#[test]
fn quantization_negative_values() {
    let original = vec![-1.0, -0.5, 0.0, 0.5, 1.0];
    let quantized = QuantizedVector::from_f32(&original);
    let dequantized = quantized.to_f32();
    for (o, d) in original.iter().zip(dequantized.iter()) {
        let error = (o - d).abs();
        assert!(error < 0.01, "Error too large: {} vs {}", o, d);
    }
}

#[test]
fn quantization_pq_roundtrip() {
    let original: Vec<f32> = (0..32).map(|i| i as f32 * 0.1).collect();
    let pq = QuantizedVector::from_f32_with_config(
        &original,
        &piramid::config::QuantizationConfig::pq(4),
    );
    let restored = pq.to_f32();
    assert_eq!(restored.len(), original.len());
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

    assert!(corrupt.try_to_f32().is_err());
}
