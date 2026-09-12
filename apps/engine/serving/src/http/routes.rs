use std::sync::Arc;

use axum::http::HeaderValue;
use axum::{
    extract::DefaultBodyLimit,
    middleware,
    routing::{delete, get, post},
    Router,
};
use tower_http::cors::{Any, CorsLayer};
use tower_http::set_header::SetResponseHeaderLayer;

use super::auth::require_api_key;
use super::handlers;
use super::rate_limit::RateLimit;
use super::request_id::assign_request_id;
use crate::state::SharedState;

/// Every API route except liveness and readiness.
fn api_router(state: SharedState) -> Router<SharedState> {
    Router::new()
        .route("/health/embeddings", get(handlers::health_embeddings))
        .route("/metrics", get(handlers::metrics))
        .route("/version", get(handlers::version))
        .route("/collections", get(handlers::list_collections))
        .route("/collections", post(handlers::create_collection))
        .route("/collections/{collection}", get(handlers::get_collection))
        .route(
            "/collections/{collection}",
            delete(handlers::delete_collection),
        )
        .route(
            "/collections/{collection}/count",
            get(handlers::collection_count),
        )
        .route(
            "/collections/{collection}/index/stats",
            get(handlers::index_stats),
        )
        .route(
            "/collections/{collection}/index/rebuild",
            post(handlers::rebuild_index),
        )
        .route(
            "/collections/{collection}/index/rebuild/status",
            get(handlers::rebuild_index_status),
        )
        .route(
            "/collections/{collection}/compact",
            post(handlers::compact_collection),
        )
        .route(
            "/collections/{collection}/duplicates",
            post(handlers::find_duplicates),
        )
        .route("/config", get(handlers::config_status))
        .route("/config/reload", post(handlers::reload_config))
        .route(
            "/collections/{collection}/vectors",
            get(handlers::list_vectors),
        )
        .route(
            "/collections/{collection}/vectors",
            post(handlers::insert_vector),
        )
        .route(
            "/collections/{collection}/vectors",
            delete(handlers::delete_vectors),
        )
        .route(
            "/collections/{collection}/vectors/{id}",
            get(handlers::get_vector),
        )
        .route(
            "/collections/{collection}/vectors/{id}",
            delete(handlers::delete_vector),
        )
        .route(
            "/collections/{collection}/upsert",
            post(handlers::upsert_vector),
        )
        // The query vector goes in the request body.
        .route(
            "/collections/{collection}/search",
            post(handlers::search_vectors),
        )
        .route(
            "/collections/{collection}/search/range",
            post(handlers::range_search_vectors),
        )
        .route(
            "/collections/{collection}/embed",
            post(handlers::embed_text),
        )
        .route(
            "/collections/{collection}/search/text",
            post(handlers::search_by_text),
        )
        .with_state(state)
}

/// Build the router: API routes under /api, the Prometheus endpoint, and middleware.
///
/// When the process booted with an API key, every route except /api/health and /api/readyz
/// requires it. A rate limit keys on peer addresses, so the router is served with connect info.
pub fn create_router(state: SharedState, rate_limit: Option<&RateLimit>) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // The API is mounted at one prefix, with no version segment.
    let mut router = Router::<SharedState>::new()
        .nest("/api", api_router(state.clone()))
        // The Prometheus endpoint sits outside the API prefix.
        .route("/metrics", get(handlers::prometheus_metrics));
    if let Some(key) = state.http_config().auth.api_key.clone() {
        router = router.route_layer(middleware::from_fn_with_state(
            Arc::new(key),
            require_api_key,
        ));
    }
    // Routes added after the authentication layer are served without a key.
    let mut router = router
        .route("/api/health", get(handlers::health))
        .route("/api/readyz", get(handlers::readyz))
        .layer(DefaultBodyLimit::max(100 * 1024 * 1024)) // 100MB for batch operations
        .layer(cors);
    if let Some(rate_limit) = rate_limit {
        router = router.layer(rate_limit.layer());
    }
    router
        .layer(middleware::from_fn(assign_request_id))
        .layer(SetResponseHeaderLayer::if_not_present(
            axum::http::header::HeaderName::from_static("x-api-version"),
            HeaderValue::from_static("v1"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            axum::http::header::HeaderName::from_static("x-content-type-options"),
            HeaderValue::from_static("nosniff"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            axum::http::header::HeaderName::from_static("x-frame-options"),
            HeaderValue::from_static("DENY"),
        ))
        .with_state(state)
}
