// Filtered search - vector similarity search with metadata filtering

use crate::storage::Collection;
use crate::metrics::Metric;
use crate::query::Filter;
use crate::search::Hit;
use crate::search::utils::sort_and_truncate;

// Perform filtered vector similarity search
// 
// Combines vector similarity search with metadata filtering.
// Note: Filtering is applied post-search, so we search for more candidates
// (k * 10) to ensure we get k results after filtering.
// 
// # Arguments
// * `storage` - The vector storage to search in
// * `query` - Query vector
// * `k` - Number of results to return (after filtering)
// * `metric` - Distance/similarity metric to use
// * `filter` - Metadata filter to apply
// 
// # Returns
// Vector of k most similar results matching the filter, sorted by score
pub fn filtered_search(
    storage: &Collection,
    query: &[f32],
    k: usize,
    metric: Metric,
    filter: &Filter,
) -> Vec<Hit> {
    // Search for more candidates to compensate for filtered-out results
    let search_k = k * 10;
    
    let results = storage.search(query, search_k, metric);

    // Apply filter
    let mut filtered: Vec<Hit> = results
        .into_iter()
        .filter_map(|hit| {
            // Get entry to check metadata
            storage.get(&hit.id).and_then(|entry| {
                if !filter.matches(&entry.metadata) {
                    return None;
                }
                Some(hit)
            })
        })
        .collect();

    // Sort and truncate
    sort_and_truncate(&mut filtered, k);
    filtered
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Document;
    use crate::metadata;

    #[test]
    fn test_filtered_search() {
        let test_db = ".piramid/tests/test_filtered_search.db";
        let test_index = ".piramid/tests/test_filtered_search.db.index.db";
        let test_wal = ".piramid/tests/test_filtered_search.db.wal.db";
        let test_vecindex = ".piramid/tests/test_filtered_search.db.vecindex.db";
        
        // Clean up any existing files
        let _ = std::fs::remove_file(test_db);
        let _ = std::fs::remove_file(test_index);
        let _ = std::fs::remove_file(test_wal);
        let _ = std::fs::remove_file(test_vecindex);
        
        {
            let mut storage = Collection::open(test_db).unwrap();

            // Insert vectors with metadata
            let e1 = Document::with_metadata(
                vec![1.0, 0.0, 0.0],
                "rust doc".to_string(),
                metadata::metadata([("lang", "rust".into())])
            );
            let e2 = Document::with_metadata(
                vec![0.9, 0.1, 0.0],
                "python doc".to_string(),
                metadata::metadata([("lang", "python".into())])
            );
            
            storage.insert(e1).unwrap();
            storage.insert(e2).unwrap();

            // Search with filter
            let filter = Filter::new().eq("lang", "rust");
            let query = vec![1.0, 0.0, 0.0];
            let results = filtered_search(&storage, &query, 5, Metric::Cosine, &filter);

            assert_eq!(results.len(), 1, "Expected 1 result, got {}: {:?}", results.len(), results.iter().map(|r| &r.text).collect::<Vec<_>>());
            assert_eq!(results[0].text, "rust doc");
        }

        // Clean up after storage is dropped
        std::fs::remove_file(test_db).unwrap();
        std::fs::remove_file(test_index).unwrap();
        let _ = std::fs::remove_file(test_wal);
        let _ = std::fs::remove_file(test_vecindex);
    }
}
