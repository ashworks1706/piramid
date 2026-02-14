use crate::metrics::Metric;
use crate::search::Hit;
use crate::storage::Collection;

pub fn search(
    collection: &Collection,
    query: &[f32],
    k: usize,
    metric: Metric,
    mut params: crate::search::SearchParams,
) -> Vec<Hit> {
    // If the execution mode in the search parameters is set to Auto, we override it with the collection's configured execution mode. 
    if matches!(params.mode, crate::config::ExecutionMode::Auto) {
        params.mode = collection.config().execution;
    }
    // If the filter overfetch override is not set in the search parameters, we set it to the collection's configured filter overfetch value.
    if params.filter_overfetch_override.is_none() {
        params.filter_overfetch_override = Some(collection.config.search.filter_overfetch);
    }
    crate::search::search_collection(collection, query, k, metric, params)
}

pub fn search_batch(
    collection: &Collection,
    queries: &[Vec<f32>],
    k: usize,
    metric: Metric,
) -> Vec<Vec<Hit>> {
    let params = crate::search::SearchParams {
        mode: collection.config().execution,
        filter: None,
        filter_overfetch_override: None,
        search_config_override: None,
    };
    crate::search::search_batch_collection(collection, queries, k, metric, params)
}
