//! Input validation for vectors, text, names, and batch sizes.

use crate::error::{Result, ServerError};

/// Reject empty vectors and any non-finite component.
pub fn validate_vector(vector: &[f32]) -> Result<()> {
    if vector.is_empty() {
        return Err(ServerError::InvalidRequest("Vector cannot be empty".to_string()).into());
    }

    for (i, &value) in vector.iter().enumerate() {
        if value.is_nan() {
            return Err(
                ServerError::InvalidRequest(format!("Vector contains NaN at index {i}")).into(),
            );
        }
        if value.is_infinite() {
            return Err(ServerError::InvalidRequest(format!(
                "Vector contains Infinity at index {i}"
            ))
            .into());
        }
    }

    Ok(())
}

/// Validate every vector in a batch.
pub fn validate_vectors(vectors: &[Vec<f32>]) -> Result<()> {
    for (i, vector) in vectors.iter().enumerate() {
        validate_vector(vector)
            .map_err(|e| ServerError::InvalidRequest(format!("Vector at index {i}: {e}")))?;
    }
    Ok(())
}

/// Reject a vector with zero magnitude, which has no cosine similarity to anything.
pub fn validate_cosine_magnitude(vector: &[f32]) -> Result<()> {
    if vector.iter().map(|&x| x * x).sum::<f32>() == 0.0 {
        return Err(ServerError::InvalidRequest(
            "Vector has zero magnitude, cosine similarity is undefined".to_string(),
        )
        .into());
    }
    Ok(())
}

/// Scale a vector to unit length. A zero or non-finite magnitude is an error.
pub fn normalize_vector(vector: &[f32]) -> Result<Vec<f32>> {
    let magnitude: f32 = vector.iter().map(|&x| x * x).sum::<f32>().sqrt();

    if magnitude == 0.0 || !magnitude.is_finite() {
        return Err(ServerError::InvalidRequest(
            "vector has zero or non-finite magnitude and cannot be normalized".to_string(),
        )
        .into());
    }

    Ok(vector.iter().map(|&x| x / magnitude).collect())
}

/// Check a vector against the collection's dimensionality.
pub fn validate_dimensions(vector: &[f32], expected_dim: usize) -> Result<()> {
    if vector.len() != expected_dim {
        return Err(ServerError::InvalidRequest(format!(
            "Vector dimension mismatch: expected {}, got {}",
            expected_dim,
            vector.len()
        ))
        .into());
    }
    Ok(())
}

/// Reject text above the size limit.
pub fn validate_text(text: &str) -> Result<()> {
    if text.is_empty() {
        return Err(ServerError::InvalidRequest("Text cannot be empty".to_string()).into());
    }

    if text.len() > 1_000_000 {
        return Err(ServerError::InvalidRequest(format!(
            "Text too large: {} bytes (max 1MB)",
            text.len()
        ))
        .into());
    }

    Ok(())
}

/// Restricted to characters safe as a filename stem on every platform.
pub fn validate_collection_name(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(
            ServerError::InvalidRequest("Collection name cannot be empty".to_string()).into(),
        );
    }

    if name.len() > 255 {
        return Err(ServerError::InvalidRequest(
            "Collection name too long (max 255 chars)".to_string(),
        )
        .into());
    }

    if !name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
    {
        return Err(ServerError::InvalidRequest(
            "Collection name can only contain alphanumeric characters, underscores, and hyphens"
                .to_string(),
        )
        .into());
    }

    Ok(())
}

/// Reject an empty batch and a batch above max_size.
pub fn validate_batch_size(size: usize, max_size: usize, operation: &str) -> Result<()> {
    if size == 0 {
        return Err(
            ServerError::InvalidRequest(format!("{operation} batch cannot be empty")).into(),
        );
    }

    if size > max_size {
        return Err(ServerError::InvalidRequest(format!(
            "{operation} batch too large: {size} items (max {max_size})"
        ))
        .into());
    }

    Ok(())
}
