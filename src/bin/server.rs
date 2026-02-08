// All the real logic lives in the `server` module.

use std::sync::Arc;
use piramid::server::{AppState, create_router};
use piramid::{EmbeddingConfig, embeddings};
use std::time::Duration;

#[tokio::main]
async fn main() {
    println!(" Piramid Vector Database");
    println!("   Version: {}", env!("CARGO_PKG_VERSION"));
    println!();
    
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
                println!("✓ Embeddings:  ENABLED");
                println!("  Provider:    {}", provider);
                println!("  Model:       {}", embedder.model_name());
                println!();
                Arc::new(AppState::with_embedder(&data_dir, embedder))
            }
            Err(e) => {
                eprintln!("✗ Embeddings:  FAILED");
                eprintln!("  Error:       {}", e);
                eprintln!("  Status:      Running without embedding support");
                eprintln!();
                Arc::new(AppState::new(&data_dir))
            }
        }
    } else {
        println!("○ Embeddings:  DISABLED");
        println!("  Configure EMBEDDING_PROVIDER to enable");
        println!();
        Arc::new(AppState::new(&data_dir))
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

    // Give in-flight requests 10 seconds to complete
    tokio::select! {
        result = server => {
            if let Err(e) = result {
                eprintln!("Server error: {}", e);
            }
        }
        _ = tokio::time::sleep(std::time::Duration::from_secs(10)) => {
            println!("   ⚠️  Force shutdown after 10s timeout");
        }
    }
    
    println!("👋 Goodbye!");
}

