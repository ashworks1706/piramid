// Helper utilities for search operations

use std::collections::HashMap;
use uuid::Uuid;

use crate::storage::{VectorStorage, VectorEntry};
use crate::metrics::Metric;
use crate::search::SearchResult;

// Create a vector map from storage (used by index for searching)
// This is a common operation needed by all search types
pub(crate) fn create_vector_map(storage: &VectorStorage) -> HashMap<Uuid, Vec<f32>> {
    storage.get_vectors()
}

// Convert a VectorEntry to SearchResult with calculated score
pub(crate) fn entry_to_result(
    entry: &VectorEntry,
    query: &[f32],
    metric: Metric,
) -> SearchResult {
    let vec = entry.get_vector();  // Dequantize
    let score = metric.calculate(query, &vec);
    SearchResult::new(
        entry.id,
        score,
        entry.text.clone(),
        vec,
        entry.metadata.clone(),
    )
}

// Sort search results by score (descending) and truncate to k
pub(crate) fn sort_and_truncate(results: &mut Vec<SearchResult>, k: usize) {
    results.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    results.truncate(k);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metadata::Metadata;
    use crate::storage::VectorEntry;
    use crate::VectorStorage;
    use crate::metrics::Metric;
    use crate::QuantizedVector;

    #[test]
    fn test_entry_to_result() {
        let entry = VectorEntry {
            id: Uuid::new_v4(),
            vector: QuantizedVector::from_f32(&vec![1.0, 0.0]),
            text: "test".to_string(),
            metadata: Metadata::new(),
        };
        
        let query = vec![1.0, 0.0];
        let result = entry_to_result(&entry, &query, Metric::Cosine);
        
        assert_eq!(result.text, "test");
        assert!(result.score > 0.99); // Cosine of identical vectors ≈ 1.0
    }

    #[test]
    fn test_sort_and_truncate() {
        let mut results = vec![
            SearchResult::new(
                Uuid::new_v4(),
                0.5,
                "low".to_string(),
                vec![],
                Metadata::new()
            ),
            SearchResult::new(
                Uuid::new_v4(),
                0.9,
                "high".to_string(),
                vec![],
                Metadata::new()
            ),
            SearchResult::new(
                Uuid::new_v4(),
                0.7,
                "mid".to_string(),
                vec![],
                Metadata::new()
            ),
        ];

        sort_and_truncate(&mut results, 2);

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].text, "high"); // Highest score first
        assert_eq!(results[1].text, "mid");
    }
}
