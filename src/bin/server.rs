// All the real logic lives in the `server` module.

use std::sync::Arc;
use piramid::server::{AppState, create_router};
use piramid::embeddings;
use piramid::config::loader::RuntimeConfig;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    // Structured logging with env-based filter (e.g., RUST_LOG=info,piramid=debug)
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_target(false)
        .init();

    println!(" Piramid Vector Database");
    println!("   Version: {}", env!("CARGO_PKG_VERSION"));
    println!();
    
    // Load config once (validated and logged)
    let RuntimeConfig {
        app: app_config,
        port,
        data_dir,
        slow_query_ms,
        embedding: embedding_config,
    } = piramid::config::loader::load_runtime_config();
    
    // Create shared state with optional embedder
    let state = if let Some(config) = embedding_config.clone() {
        match embeddings::providers::create_embedder(&config) {
            Ok(embedder) => {
                println!("✓ Embeddings:  ENABLED");
                println!("  Provider:    {}", config.provider);
                println!("  Model:       {}", embedder.model_name());
                println!();
                
                // Wrap with retry logic (3 retries, exponential backoff)
                let retry_embedder = Arc::new(embeddings::RetryEmbedder::new(embedder));
                Arc::new(AppState::with_embedder(&data_dir, app_config.clone(), slow_query_ms, retry_embedder))
            }
            Err(e) => {
                eprintln!("✗ Embeddings:  FAILED");
                eprintln!("  Error:       {}", e);
                eprintln!("  Status:      Running without embedding support");
                eprintln!();
                Arc::new(AppState::new(&data_dir, app_config.clone(), slow_query_ms))
            }
        }
    } else {
        println!("○ Embeddings:  DISABLED");
        println!("  Configure EMBEDDING_PROVIDER to enable");
        println!();
        Arc::new(AppState::new(&data_dir, app_config.clone(), slow_query_ms))
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
