use piramid::quantization::QuantizedVector;

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
