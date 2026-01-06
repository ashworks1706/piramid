//! Piramid Server - the main entry point
//!
//! This is what you run: `cargo run --bin piramid-server`
//!
//! It's intentionally minimal - just config and startup.
//! All the real logic lives in the `server` module.

use std::sync::Arc;
use piramid::server::{AppState, create_router};

#[tokio::main]
async fn main() {
    // Config from environment (with sensible defaults)
    let port = std::env::var("PORT").unwrap_or_else(|_| "6333".to_string());
    let data_dir = std::env::var("DATA_DIR").unwrap_or_else(|_| "./piramid_data".to_string());
    
    // Create shared state
    let state = Arc::new(AppState::new(&data_dir));
    
    // Build router with all our routes
    let app = create_router(state);
    
    // Start listening
    let addr = format!("0.0.0.0:{}", port);
    println!("🔺 Piramid server running on http://{}", addr);
    println!("   Data directory: {}", data_dir);
    
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
