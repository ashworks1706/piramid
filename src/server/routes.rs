// This is the "routing table" of our API.
// - GET    = read (list, get one)
// - POST   = create or action (store, search)
// - DELETE = remove
// - PUT    = replace (not used yet)
// - PATCH  = partial update (not used yet)

use axum::{
    extract::DefaultBodyLimit,
    routing::{delete, get, post},
    Router,
    middleware,
};
use axum::http::HeaderValue;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::{ServeDir, ServeFile};
use tower_http::set_header::SetResponseHeaderLayer;

use super::handlers;
use super::state::SharedState;
use super::request_id::assign_request_id;

fn api_router(state: SharedState) -> Router<SharedState> {
    Router::new()
        // Health and metrics endpoints
        .route("/health", get(handlers::health))
        .route("/health/embeddings", get(handlers::health_embeddings))
        .route("/readyz", get(handlers::readyz))
        .route("/metrics", get(handlers::metrics))
        .route("/version", get(handlers::version))
        
        // Collections CRUD
        .route("/collections", get(handlers::list_collections))
        .route("/collections", post(handlers::create_collection))
        .route("/collections/{collection}", get(handlers::get_collection))
        .route("/collections/{collection}", delete(handlers::delete_collection))
        .route("/collections/{collection}/count", get(handlers::collection_count))
        .route("/collections/{collection}/index/stats", get(handlers::index_stats))
        .route("/collections/{collection}/index/rebuild", post(handlers::rebuild_index))
        .route("/collections/{collection}/index/rebuild/status", get(handlers::rebuild_index_status))
        
        // Config hot reload/status
        .route("/config", get(handlers::config_status))
        .route("/config/reload", post(handlers::reload_config))
        
        // Vectors CRUD
        .route("/collections/{collection}/vectors", get(handlers::list_vectors))
        .route("/collections/{collection}/vectors", post(handlers::insert_vector))
        .route("/collections/{collection}/vectors", delete(handlers::delete_vectors))
        .route("/collections/{collection}/vectors/{id}", get(handlers::get_vector))
        .route("/collections/{collection}/vectors/{id}", delete(handlers::delete_vector))
        
        // Upsert
        .route("/collections/{collection}/upsert", post(handlers::upsert_vector))
        
        // Search (POST because we're sending a vector in body)
        .route("/collections/{collection}/search", post(handlers::search_vectors))
        .route("/collections/{collection}/search/range", post(handlers::range_search_vectors))

        // Embedding endpoints
        .route("/collections/{collection}/embed", post(handlers::embed_text))
        .route("/collections/{collection}/search/text", post(handlers::search_by_text))
        .with_state(state)
}

// This function wires everything together:
// 1. Creates route definitions
// 2. Adds CORS middleware 
// 3. Attaches shared state
pub fn create_router(state: SharedState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)    // any domain can call us
        .allow_methods(Any)   // GET, POST, etc
        .allow_headers(Any);  // any headers

    let api = api_router();
    
    Router::new()
        .with_state(state.clone())
        .nest("/api", api.clone())
        .nest("/api/v1", api)
        // Middleware layers
        .layer(DefaultBodyLimit::max(100 * 1024 * 1024))  // 100MB for batch operations
        .layer(cors)
        // Assign request IDs to all requests
        .layer(middleware::from_fn(assign_request_id))
        // Add API version header
        .layer(middleware::map_response(|mut res: Response| {
            res.headers_mut().insert(
                "X-API-Version",
                HeaderValue::from_static("v1"),
            );
            res
        }))
        // Serve static dashboard files (Next.js export)
        .fallback_service(
            ServeDir::new("dashboard")
                .not_found_service(ServeFile::new("dashboard/index.html"))
        )
}
