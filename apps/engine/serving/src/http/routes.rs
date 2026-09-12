//! The route table and the middleware stack of the server.

use axum::http::HeaderValue;
use axum::{
    extract::DefaultBodyLimit,
    middleware,
    routing::{delete, get, post},
    Router,
};
use tower_http::cors::{Any, CorsLayer};
use tower_http::set_header::SetResponseHeaderLayer;

use super::handlers;
use super::request_id::assign_request_id;
use crate::state::SharedState;

fn api_router(state: SharedState) -> Router<SharedState> {
    Router::new()
        .route("/health", get(handlers::health))
        .route("/health/embeddings", get(handlers::health_embeddings))
        .route("/readyz", get(handlers::readyz))
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
pub fn create_router(state: SharedState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // The API is mounted at one prefix, with no version segment.
    Router::<SharedState>::new()
        .nest("/api", api_router(state.clone()))
        // The Prometheus endpoint sits outside the API prefix.
        .route("/metrics", get(handlers::prometheus_metrics))
        .layer(DefaultBodyLimit::max(100 * 1024 * 1024)) // 100MB for batch operations
        .layer(cors)
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
