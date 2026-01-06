//! Route definitions - maps URLs to handlers
//!
//! This is the "routing table" of our API.
//! axum uses a builder pattern: Router::new().route(...).route(...)
//!
//! REST conventions we follow:
//! - GET    = read (list, get one)
//! - POST   = create or action (store, search)
//! - DELETE = remove
//! - PUT    = replace (not used yet)
//! - PATCH  = partial update (not used yet)

use axum::{
    routing::{delete, get, post},
    Router,
};
use tower_http::cors::{Any, CorsLayer};

use super::handlers;
use super::state::SharedState;

/// Build the complete router with all endpoints
/// 
/// This function wires everything together:
/// 1. Creates route definitions
/// 2. Adds CORS middleware (so browsers can call us)
/// 3. Attaches shared state
pub fn create_router(state: SharedState) -> Router {
    // CORS = Cross-Origin Resource Sharing
    // Without this, browsers block requests from different domains
    // (e.g., dashboard on :3000 calling API on :6333)
    let cors = CorsLayer::new()
        .allow_origin(Any)    // any domain can call us
        .allow_methods(Any)   // GET, POST, etc
        .allow_headers(Any);  // any headers
    
    Router::new()
        // Health check - always first, it's what load balancers hit
        .route("/api/health", get(handlers::health))
        
        // Collections CRUD
        .route("/api/collections", get(handlers::list_collections))
        .route("/api/collections", post(handlers::create_collection))
        .route("/api/collections/{name}", get(handlers::get_collection))
        .route("/api/collections/{name}", delete(handlers::delete_collection))
        .route("/api/collections/{name}/count", get(handlers::collection_count))
        
        // Vectors CRUD
        .route("/api/collections/{collection}/vectors", get(handlers::list_vectors))
        .route("/api/collections/{collection}/vectors", post(handlers::store_vector))
        .route("/api/collections/{collection}/vectors/{id}", get(handlers::get_vector))
        .route("/api/collections/{collection}/vectors/{id}", delete(handlers::delete_vector))
        
        // Search (POST because we're sending a vector in body)
        .route("/api/collections/{collection}/search", post(handlers::search_vectors))
        
        // Middleware layers
        .layer(cors)
        // State available to all handlers
        .with_state(state)
}
