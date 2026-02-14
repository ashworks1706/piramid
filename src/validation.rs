// Input validation and sanitization for vectors and requests

use crate::error::{Result, ServerError};

// Validate vector format (check for NaN, Infinity)
pub fn validate_vector(vector: &[f32]) -> Result<()> {
    if vector.is_empty() {
        return Err(ServerError::InvalidRequest("Vector cannot be empty".to_string()).into());
    }
    
    for (i, &value) in vector.iter().enumerate() {
        if value.is_nan() {
            return Err(ServerError::InvalidRequest(
                format!("Vector contains NaN at index {}", i)
            ).into());
        }
        if value.is_infinite() {
            return Err(ServerError::InvalidRequest(
                format!("Vector contains Infinity at index {}", i)
            ).into());
        }
    }
    
    Ok(())
}

// Validate multiple vectors
pub fn validate_vectors(vectors: &[Vec<f32>]) -> Result<()> {
    for (i, vector) in vectors.iter().enumerate() {
        validate_vector(vector)
            .map_err(|e| ServerError::InvalidRequest(
                format!("Vector at index {}: {}", i, e)
            ))?;
    }
    Ok(())
}

// Normalize a vector to unit length (L2 normalization)
// Useful for cosine similarity
// why do production vector dbs need to noramlize vector ?
// Normalization ensures consistent similarity calculations, especially for cosine similarity which
// relies on vector direction rather than magnitude. It also helps prevent numerical instability
// and improves performance by reducing the range of values.
// Why L2? because L2 normalization (dividing by the vector's magnitude) is the most common method
// for normalizing vectors in machine learning and information retrieval. It preserves the
// direction of the vector while scaling it to have a length of 1, which is ideal for similarity
// calculations. L1 normalization (dividing by the sum of absolute values) can be used in some
// cases, but L2 is generally preferred for its mathematical properties and performance benefits.
pub fn normalize_vector(vector: &[f32]) -> Vec<f32> {
    let magnitude: f32 = vector.iter().map(|&x| x * x).sum::<f32>().sqrt();
    
    if magnitude == 0.0 || magnitude.is_nan() || magnitude.is_infinite() {
        // Return zero vector if magnitude is invalid
        return vec![0.0; vector.len()];
    }
    
    vector.iter().map(|&x| x / magnitude).collect()
}

// Check if vector dimensions match expected dimensions
pub fn validate_dimensions(vector: &[f32], expected_dim: usize) -> Result<()> {
    if vector.len() != expected_dim {
        return Err(ServerError::InvalidRequest(
            format!("Vector dimension mismatch: expected {}, got {}", expected_dim, vector.len())
        ).into());
    }
    Ok(())
}

// Validate text input (basic sanitization)
pub fn validate_text(text: &str) -> Result<()> {
    if text.is_empty() {
        return Err(ServerError::InvalidRequest("Text cannot be empty".to_string()).into());
    }
    
    if text.len() > 1_000_000 {  // 1MB text limit
        return Err(ServerError::InvalidRequest(
            format!("Text too large: {} bytes (max 1MB)", text.len())
        ).into());
    }
    
    Ok(())
}

// Validate collection name
pub fn validate_collection_name(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(ServerError::InvalidRequest("Collection name cannot be empty".to_string()).into());
    }
    
    if name.len() > 255 {
        return Err(ServerError::InvalidRequest("Collection name too long (max 255 chars)".to_string()).into());
    }
    
    // Only allow alphanumeric, underscore, hyphen
    if !name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
        return Err(ServerError::InvalidRequest(
            "Collection name can only contain alphanumeric characters, underscores, and hyphens".to_string()
        ).into());
    }
    
    Ok(())
}

// Validate batch size limits
pub fn validate_batch_size(size: usize, max_size: usize, operation: &str) -> Result<()> {
    if size == 0 {
        return Err(ServerError::InvalidRequest(
            format!("{} batch cannot be empty", operation)
        ).into());
    }
    
    if size > max_size {
        return Err(ServerError::InvalidRequest(
            format!("{} batch too large: {} items (max {})", operation, size, max_size)
        ).into());
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_vector_valid() {
        assert!(validate_vector(&[1.0, 2.0, 3.0]).is_ok());
        assert!(validate_vector(&[0.0, -1.5, 100.0]).is_ok());
    }

    #[test]
    fn test_validate_vector_nan() {
        let result = validate_vector(&[1.0, f32::NAN, 3.0][..]);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("NaN"));
    }

    #[test]
    fn test_validate_vector_infinity() {
        let result = validate_vector(&[1.0, f32::INFINITY, 3.0][..]);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Infinity"));
    }

    #[test]
    fn test_validate_vector_empty() {
        let result = validate_vector(&[][..]);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("empty"));
    }

    #[test]
    fn test_normalize_vector() {
        let vec = vec![3.0, 4.0];
        let normalized = normalize_vector(&vec);
        
        // Length should be 1
        let magnitude: f32 = normalized.iter().map(|&x| x * x).sum::<f32>().sqrt();
        assert!((magnitude - 1.0).abs() < 0.0001);
        
        // Values should be 0.6 and 0.8
        assert!((normalized[0] - 0.6).abs() < 0.0001);
        assert!((normalized[1] - 0.8).abs() < 0.0001);
    }

    #[test]
    fn test_normalize_zero_vector() {
        let vec = vec![0.0, 0.0];
        let normalized = normalize_vector(&vec);
        assert_eq!(normalized, vec![0.0, 0.0]);
    }

    #[test]
    fn test_validate_dimensions() {
        assert!(validate_dimensions(&[1.0, 2.0, 3.0], 3).is_ok());
        assert!(validate_dimensions(&[1.0, 2.0], 3).is_err());
        assert!(validate_dimensions(&[1.0, 2.0, 3.0, 4.0], 3).is_err());
    }

    #[test]
    fn test_validate_collection_name() {
        assert!(validate_collection_name("my_collection").is_ok());
        assert!(validate_collection_name("test-123").is_ok());
        assert!(validate_collection_name("").is_err());
        assert!(validate_collection_name("invalid name").is_err());
        assert!(validate_collection_name("invalid@name").is_err());
    }

    #[test]
    fn test_validate_batch_size() {
        assert!(validate_batch_size(10, 100, "insert").is_ok());
        assert!(validate_batch_size(0, 100, "insert").is_err());
        assert!(validate_batch_size(101, 100, "insert").is_err());
    }
}
