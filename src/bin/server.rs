//! Piramid Server - the main entry point
//! All the real logic lives in the `server` module.

use std::sync::Arc;
use piramid::server::{AppState, create_router};
use piramid::{EmbeddingConfig, embeddings};

#[tokio::main]
async fn main() {
    // Config from environment (with sensible defaults)
    let port = std::env::var("PORT").unwrap_or_else(|_| "6333".to_string());
    let data_dir = std::env::var("DATA_DIR").unwrap_or_else(|_| "./piramid_data".to_string());
    
    // Optional embedding configuration
    let embedding_provider = std::env::var("EMBEDDING_PROVIDER").ok();
    let embedding_model = std::env::var("EMBEDDING_MODEL").ok();
    
    // Create shared state with optional embedder
    let state = if let Some(provider) = embedding_provider {
        let model = embedding_model.unwrap_or_else(|| {
            if provider == "openai" {
                "text-embedding-3-small".to_string()
            } else if provider == "ollama" {
                "nomic-embed-text".to_string()
            } else {
                "text-embedding-3-small".to_string()
            }
        });

        let config = EmbeddingConfig {
            provider: provider.clone(),
            model,
            api_key: std::env::var("OPENAI_API_KEY").ok(),
            base_url: std::env::var("EMBEDDING_BASE_URL").ok(),
            options: serde_json::json!({}),
        };

        match embeddings::providers::create_embedder(&config) {
            Ok(embedder) => {
                println!("   Embedding provider: {}", provider);
                println!("   Embedding model: {}", embedder.model_name());
                Arc::new(AppState::with_embedder(&data_dir, embedder))
            }
            Err(e) => {
                eprintln!("⚠️  Failed to initialize embedder: {}", e);
                eprintln!("   Continuing without embedding support");
                Arc::new(AppState::new(&data_dir))
            }
        }
    } else {
        Arc::new(AppState::new(&data_dir))
    };
    
    // Build router with all our routes
    let app = create_router(state);
    
    // Start listening
    let addr = format!("0.0.0.0:{}", port);
    println!("🔺 Piramid server running on http://{}", addr);
    println!("   Data directory: {}", data_dir);
    
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
