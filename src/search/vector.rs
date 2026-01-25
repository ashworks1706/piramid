// Vector similarity search (k-NN)
// Find the k most similar vectors to a query vector

use crate::storage::VectorStorage;
use crate::metrics::Metric;
use crate::search::SearchResult;
use super::utils::{create_vector_map, entry_to_result, sort_and_truncate};

// Perform k-nearest neighbor vector similarity search
// 
// # Arguments
// * `storage` - The vector storage to search in
// * `query` - Query vector
// * `k` - Number of results to return
// * `metric` - Distance/similarity metric to use
// 
// # Returns
// Vector of k most similar results, sorted by score (highest first)
pub fn vector_search(
    storage: &VectorStorage,
    query: &[f32],
    k: usize,
    metric: Metric,
) -> Vec<SearchResult> {
    // Create vector map for index
    let vector_map = create_vector_map(storage);

    // Use index to find k nearest neighbors
    // ef = 2*k for better recall (HNSW parameter)
    let ef = (k * 2).max(50);
    let result_ids = storage.index().search(query, k, ef, &vector_map);

    // Convert IDs to SearchResults
    let mut results: Vec<SearchResult> = result_ids
        .into_iter()
        .filter_map(|id| {
            storage.get(&id).map(|entry| {
                entry_to_result(&entry, query, metric)
            })
        })
        .collect();

    // Sort and truncate
    sort_and_truncate(&mut results, k);
    results
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::VectorEntry;

    #[test]
    fn test_vector_search() {
        let _ = std::fs::remove_file("test_vector_search.db");
        let mut storage = VectorStorage::open("test_vector_search.db").unwrap();

        // Insert test vectors
        let vectors = vec![
            vec![1.0, 0.0, 0.0],
            vec![0.0, 1.0, 0.0],
            vec![0.9, 0.1, 0.0],
        ];

        for (i, vec) in vectors.iter().enumerate() {
            let entry = VectorEntry::new(vec.clone(), format!("doc{}", i));
            storage.store(entry).unwrap();
        }

        // Search
        let query = vec![1.0, 0.0, 0.0];
        let results = vector_search(&storage, &query, 2, Metric::Cosine);

        assert_eq!(results.len(), 2);
        assert!(results[0].text == "doc0" || results[0].text == "doc2");

        std::fs::remove_file("test_vector_search.db").unwrap();
    }
}
