// Unified search engine for collections.

use crate::config::ExecutionMode;
use crate::metrics::Metric;
use crate::search::{Hit, query::Filter, utils::sort_and_truncate};
use crate::storage::Collection;

// Parameters for a search request.
#[derive(Debug, Clone, Copy)]
pub struct SearchParams<'a> {
    pub mode: ExecutionMode,
    pub filter: Option<&'a Filter>,
    pub filter_overfetch_override: Option<usize>,
}

impl Default for SearchParams<'_> {
    fn default() -> Self {
        Self {
            mode: ExecutionMode::Auto,
            filter: None,
            filter_overfetch_override: None,
        }
    }
}

pub fn search_collection(
    storage: &Collection,
    query: &[f32],
    k: usize, 
    metric: Metric,
    params: SearchParams<'_>,
) -> Vec<Hit> {
    // Build vector map once for the index to use.
    let vectors = storage.get_vectors();
    let base_overfetch = storage.config.search.filter_overfetch.max(1);
    let expansion = params
        .filter_overfetch_override
        .unwrap_or(base_overfetch)
        .max(1);
    let search_k = if params.filter.is_some() { k.saturating_mul(expansion) } else { k };
    let mode = params.mode;

    let neighbor_ids = storage.vector_index().search(
        query,
        search_k,
        vectors,
        storage.config.search,
    );

    let mut results = Vec::new();
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

    if let Some(filter) = params.filter {
        let mut filtered = results;
        filtered.retain(|hit| filter.matches(&hit.metadata));
        sort_and_truncate(&mut filtered, k);
        filtered
    } else {
        results
    }
}

pub fn search_batch_collection(
    storage: &Collection,
    queries: &[Vec<f32>],
    k: usize,
    metric: Metric,
    params: SearchParams<'_>,
) -> Vec<Vec<Hit>> {
    if storage.config().parallelism.parallel_search {
        use rayon::prelude::*;
        queries
            .par_iter()
            .map(|query| search_collection(storage, query, k, metric, params))
            .collect()
    } else {
        queries
            .iter()
            .map(|query| search_collection(storage, query, k, metric, params))
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
