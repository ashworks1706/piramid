// Helper utilities for search operations

use crate::search::Hit;

// Sort search results by score (descending) and truncate to k
pub(crate) fn sort_and_truncate(results: &mut Vec<Hit>, k: usize) {
    results.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score) // Sort by score (descending)
            .unwrap_or(std::cmp::Ordering::Equal) // Handle NaN cases by treating them as equal
    }); // Sort by score (descending)
    results.truncate(k);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metadata::Metadata;
    use uuid::Uuid;

    #[test]
    fn test_sort_and_truncate() {
        let mut results = vec![
            Hit::new(
                Uuid::new_v4(),
                0.5,
                "low".to_string(),
                vec![],
                Metadata::new()
            ),
            Hit::new(
                Uuid::new_v4(),
                0.9,
                "high".to_string(),
                vec![],
                Metadata::new()
            ),
            Hit::new(
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
