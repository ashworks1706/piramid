// All the real logic lives in the `server` module.

use std::sync::Arc;
use piramid::server::{AppState, create_router};
use piramid::{EmbeddingConfig, embeddings};

#[tokio::main]
async fn main() {
    println!(" Piramid Vector Database");
    println!("   Version: {}", env!("CARGO_PKG_VERSION"));
    println!();
    
    // Config from environment (with sensible defaults)
    let port = std::env::var("PORT").unwrap_or_else(|_| "6333".to_string());
    let data_dir = std::env::var("DATA_DIR").unwrap_or_else(|_| "./.piramid".to_string());
    
    // Optional embedding configuration
    let embedding_provider = std::env::var("EMBEDDING_PROVIDER").ok();
    let embedding_model = std::env::var("EMBEDDING_MODEL").ok();
    let app_config = piramid::config::loader::load_app_config();
    
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
                println!("✓ Embeddings:  ENABLED");
                println!("  Provider:    {}", provider);
                println!("  Model:       {}", embedder.model_name());
                println!();
                
                // Wrap with retry logic (3 retries, exponential backoff)
                let retry_embedder = Arc::new(embeddings::RetryEmbedder::new(embedder));
                Arc::new(AppState::with_embedder(&data_dir, app_config.clone(), retry_embedder))
            }
            Err(e) => {
                eprintln!("✗ Embeddings:  FAILED");
                eprintln!("  Error:       {}", e);
                eprintln!("  Status:      Running without embedding support");
                eprintln!();
                Arc::new(AppState::new(&data_dir, app_config.clone()))
            }
        }
    } else {
        println!("○ Embeddings:  DISABLED");
        println!("  Configure EMBEDDING_PROVIDER to enable");
        println!();
        Arc::new(AppState::new(&data_dir, app_config.clone()))
    };
    
    // Build router with all our routes
    let app = create_router(state.clone());
    
    // Start listening
    let addr = format!("0.0.0.0:{}", port);
    println!("⚡ Server:      READY");
    println!("  HTTP:        http://{}", addr);
    println!("  Data:        {}", data_dir);
    println!("  Dashboard:   http://localhost:{}/", port);
    println!();
    println!("Press Ctrl+C to stop");
    
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    
    let state_for_shutdown = state.clone();
    
    // Graceful shutdown signal
    let shutdown_signal = async move {
        tokio::signal::ctrl_c().await.expect("Failed to listen for Ctrl+C");
        println!("\n⚡ Received shutdown signal...");
        
        // Set shutdown flag to reject new requests
        state_for_shutdown.initiate_shutdown();
        println!("   ⏸️  Rejecting new requests");
        
        // Flush all collections
        println!("   💾 Flushing collections...");
        if let Err(e) = state_for_shutdown.checkpoint_all() {
            eprintln!("   ❌ Error saving data during shutdown: {}", e);
        } else {
            println!("   ✅ All data saved");
        }
        
        println!("   🔌 Draining connections (10s timeout)...");
    };
    
    let server = axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal);

    // Run server until shutdown signal
    if let Err(e) = server.await {
        eprintln!("Server error: {}", e);
    }
    
    println!("👋 Goodbye!");
}
