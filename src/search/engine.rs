// Unified search engine for collections.
// Wraps vector index search + scoring and optional metadata filtering.

use crate::config::ExecutionMode;
use crate::metrics::Metric;
use crate::search::{Hit, query::Filter, utils::sort_and_truncate};
use crate::storage::Collection;
use uuid::Uuid;
use std::collections::HashMap;

// Parameters for a search request.
#[derive(Debug, Clone, Copy)]
pub struct SearchParams<'a> {
    pub mode: ExecutionMode,
    pub filter: Option<&'a Filter>,
    pub filter_overfetch_override: Option<usize>,
    pub search_config_override: Option<crate::config::SearchConfig>,
}

impl Default for SearchParams<'_> {
    fn default() -> Self {
        Self {
            mode: ExecutionMode::Auto,
            filter: None,
            filter_overfetch_override: None,
            search_config_override: None,
        }
    }
}

fn search_collection_with_maps(
    storage: &Collection,
    query: &[f32],
    k: usize,
    metric: Metric,
    params: SearchParams<'_>,
    vectors: &HashMap<Uuid, Vec<f32>>,
    metadatas: &HashMap<Uuid, crate::metadata::Metadata>,
) -> Vec<Hit> {
    // 1. Determine effective search config and overfetch factor
    let effective_search = params.search_config_override.unwrap_or(storage.config.search);

    // 2. Calculate overfetch factor based on filter presence and configuration. If a filter is applied, we need to overfetch more results from the vector index to ensure that after filtering we still have enough results to return. The overfetch factor is determined by the search configuration's filter_overfetch parameter, which specifies how many times more results to fetch compared to k when a filter is applied. If no filter is present, we can just fetch k results directly.
    let base_overfetch = effective_search.filter_overfetch.max(1);
    
    // If the caller provided an override for filter overfetch, use that instead of the configured value. This allows for dynamic adjustment of the overfetch factor on a per-search basis, which can be useful for certain queries or workloads where the default overfetch might not be sufficient or might be too aggressive.
    let expansion = params
        .filter_overfetch_override
        .unwrap_or(base_overfetch)
        .max(1);

    // 3. Perform search on the vector index with the calculated overfetch factor. If a filter is present, we multiply k by the expansion factor to fetch more results from the vector index, which increases the likelihood that after filtering we will have at least k results to return. If no filter is present, we just fetch k results directly from the vector index.
    let search_k = if params.filter.is_some() { k.saturating_mul(expansion) } else { k };
    
    // 4. Search the vector index for nearest neighbors to the query vector. This will return a list of candidate IDs based on vector similarity. The search method of the vector index will use the effective search configuration, which may include parameters like ef for HNSW or num_probes for IVF, to control the tradeoff between search speed and accuracy. The filter and metadata parameters are passed to the search method, although they may not be used by all index types.
    let mode = params.mode;


    let neighbor_ids = storage.vector_index().search(
        query,
        search_k,
        vectors,
        effective_search,
        params.filter,
        metadatas,
    );

    let mut results = Vec::new();

    // 5. For each candidate ID returned by the vector index search, retrieve the corresponding vector and metadata from storage, calculate the similarity score using the specified metric, and construct a Hit object that includes the ID, score, text, vector, and metadata. This step involves looking up each candidate ID in the storage to get the full information needed to return to the caller. The similarity score is calculated using the configured metric (e.g., cosine similarity), which takes into account the query vector and the candidate vector.
    for id in neighbor_ids {
        if let Some(entry) = storage.get(&id) {
            let vec = entry.get_vector();
            let score = metric.calculate(query, &vec, mode);
            results.push(Hit {
                id,
                score,
                text: entry.text,
                vector: vec,
                metadata: entry.metadata.clone(),
            });
        }
    }
    // 6. If a filter is provided, apply the filter to the results to retain only those hits that match the filter criteria. After filtering, sort the results by score and truncate to the top k results to return to the caller. If no filter is provided, we can skip this step and just return the results as they are already sorted by the vector index search.
    if let Some(filter) = params.filter {
        let mut filtered = results;
        filtered.retain(|hit| filter.matches(&hit.metadata));
        sort_and_truncate(&mut filtered, k);
        filtered
    } else {
        results
    }
}

pub fn search_collection(
    storage: &Collection,
    query: &[f32],
    k: usize, 
    metric: Metric,
    params: SearchParams<'_>,
) -> Vec<Hit> {
    // Get vectors and metadatas from storage to pass to the search function. This allows us to perform the search using the vector index while also having access to the metadata for filtering and constructing the Hit objects. The search_collection_with_maps function is then called with these maps to perform the actual search and return the results.
    let vectors = storage.get_vectors();
    let metadatas = storage.metadata_view();
    search_collection_with_maps(storage, query, k, metric, params, vectors, metadatas)
}

pub fn search_batch_collection(
    storage: &Collection,
    queries: &[Vec<f32>],
    k: usize,
    metric: Metric,
    params: SearchParams<'_>,
) -> Vec<Vec<Hit>> {
    let vectors = storage.get_vectors();
    let metadatas = storage.metadata_view();
    
    if storage.config().parallelism.parallel_search {
        use rayon::prelude::*; // If parallel search is enabled in the configuration, we use Rayon to perform the searches for each query in parallel. This can significantly speed up batch searches when there are multiple queries and the underlying hardware supports parallel execution. Each query is processed independently, and the results are collected into a vector of vectors of hits, where each inner vector corresponds to the results for a single query.
        queries
            .par_iter()
            .map(|query| search_collection_with_maps(storage, query, k, metric, params, vectors, metadatas))
            .collect() 
    } else {
        queries
            .iter()
            .map(|query| search_collection_with_maps(storage, query, k, metric, params, vectors, metadatas))
            .collect() 
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metadata;
    use crate::storage::Document;
    use crate::metrics::Metric;

    #[test]
    fn search_respects_filter() {
        let test_db = ".piramid/tests/test_search_filter.db";
        let test_index = ".piramid/tests/test_search_filter.db.index.db";
        let test_wal = ".piramid/tests/test_search_filter.db.wal.db";
        let test_vecindex = ".piramid/tests/test_search_filter.db.vecindex.db";

        let _ = std::fs::remove_file(test_db);
        let _ = std::fs::remove_file(test_index);
        let _ = std::fs::remove_file(test_wal);
        let _ = std::fs::remove_file(test_vecindex);

        {
            let mut storage = Collection::open(test_db).unwrap();

            let e1 = Document::with_metadata(
                vec![1.0, 0.0, 0.0],
                "rust doc".to_string(),
                metadata::metadata([("lang", "rust".into())]),
            );
            let e2 = Document::with_metadata(
                vec![0.9, 0.1, 0.0],
                "python doc".to_string(),
                metadata::metadata([("lang", "python".into())]),
            );

            storage.insert(e1).unwrap();
            storage.insert(e2).unwrap();

            let filter = crate::search::Filter::new().eq("lang", "rust");
            let params = SearchParams {
                mode: storage.config().execution,
                filter: Some(&filter),
                filter_overfetch_override: None,
                search_config_override: None,
            };

            let results = search_collection(&storage, &[1.0, 0.0, 0.0][..], 5, Metric::Cosine, params);
            assert_eq!(results.len(), 1);
            assert_eq!(results[0].text, "rust doc");
        }

        let _ = std::fs::remove_file(test_db);
        let _ = std::fs::remove_file(test_index);
        let _ = std::fs::remove_file(test_wal);
        let _ = std::fs::remove_file(test_vecindex);
    }
}
