// Vector similarity search (k-NN)
// Find the k most similar vectors to a query vector

use crate::storage::Collection;
use crate::metrics::Metric;
use crate::search::Hit;

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
    storage: &Collection,
    query: &[f32],
    k: usize,
    metric: Metric,
) -> Vec<Hit> {
    // Use storage's search method which handles all index types
    storage.search(query, k, metric)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Document;

    #[test]
    fn test_vector_search() {
        let _ = std::fs::remove_file(".piramid/tests/test_vector_search.db");
        let _ = std::fs::remove_file(".piramid/tests/test_vector_search.db.index.db");
        let _ = std::fs::remove_file(".piramid/tests/test_vector_search.db.wal.db");
        let _ = std::fs::remove_file(".piramid/tests/test_vector_search.db.vecindex.db");
        
        let mut storage = Collection::open(".piramid/tests/test_vector_search.db").unwrap();

        // Insert test vectors
        let vectors = vec![
            vec![1.0, 0.0, 0.0],
            vec![0.0, 1.0, 0.0],
            vec![0.9, 0.1, 0.0],
        ];

        for (i, vec) in vectors.iter().enumerate() {
            let entry = Document::new(vec.clone(), format!("doc{}", i));
            storage.insert(entry).unwrap();
        }

        // Search
        let query = vec![1.0, 0.0, 0.0];
        let results = vector_search(&storage, &query, 2, Metric::Cosine);

        assert_eq!(results.len(), 2);
        assert!(results[0].text == "doc0" || results[0].text == "doc2");

        drop(storage);
        std::fs::remove_file(".piramid/tests/test_vector_search.db").unwrap();
        let _ = std::fs::remove_file(".piramid/tests/test_vector_search.db.index.db");
        let _ = std::fs::remove_file(".piramid/tests/test_vector_search.db.wal.db");
        let _ = std::fs::remove_file(".piramid/tests/test_vector_search.db.vecindex.db");
    }
}
