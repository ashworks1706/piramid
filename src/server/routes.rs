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
};
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::{ServeDir, ServeFile};

use super::handlers;
use super::state::SharedState;

// This function wires everything together:
// 1. Creates route definitions
// 2. Adds CORS middleware 
// 3. Attaches shared state
pub fn create_router(state: SharedState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)    // any domain can call us
        .allow_methods(Any)   // GET, POST, etc
        .allow_headers(Any);  // any headers
    
    Router::new()
        // Health check - always first, it's what load balancers hit
        .route("/api/health", get(handlers::health))
        .route("/api/health/embeddings", get(handlers::health_embeddings))
        .route("/api/metrics", get(handlers::metrics))
        
        // Collections CRUD
        .route("/api/collections", get(handlers::list_collections))
        .route("/api/collections", post(handlers::create_collection))
        .route("/api/collections/{name}", get(handlers::get_collection))
        .route("/api/collections/{name}", delete(handlers::delete_collection))
        .route("/api/collections/{name}/count", get(handlers::collection_count))
        
        // Vectors CRUD
        .route("/api/collections/{collection}/vectors", get(handlers::list_vectors))
        .route("/api/collections/{collection}/vectors", post(handlers::insert_vector))
        .route("/api/collections/{collection}/vectors/batch", post(handlers::insert_vectors_batch))
        .route("/api/collections/{collection}/vectors/batch", delete(handlers::delete_vectors_batch))
        .route("/api/collections/{collection}/vectors/{id}", get(handlers::get_vector))
        .route("/api/collections/{collection}/vectors/{id}", delete(handlers::delete_vector))
        
        // Upsert
        .route("/api/collections/{collection}/upsert", post(handlers::upsert_vector))
        
        // Search (POST because we're sending a vector in body)
        .route("/api/collections/{collection}/search", post(handlers::search_vectors))
        .route("/api/collections/{collection}/search/batch", post(handlers::batch_search_vectors))
        
        // Embedding endpoints
        .route("/api/collections/{collection}/embed", post(handlers::embed_text))
        .route("/api/collections/{collection}/embed/batch", post(handlers::embed_batch))
        .route("/api/collections/{collection}/search/text", post(handlers::search_by_text))
        
        // Middleware layers
        .layer(DefaultBodyLimit::max(100 * 1024 * 1024))  // 100MB for batch operations
        .layer(cors)
        // State available to all handlers
        .with_state(state)
        // Serve static dashboard files (Next.js export)
        .fallback_service(
            ServeDir::new("dashboard")
                .not_found_service(ServeFile::new("dashboard/index.html"))
        )
}
