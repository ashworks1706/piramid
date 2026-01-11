//! Example demonstrating embedding functionality with Piramid
//! 
//! This example shows how to:
//! 1. Create an embedder (OpenAI or Ollama)
//! 2. Embed text to vectors
//! 3. Store and search by text

use piramid::{
    VectorStorage, VectorEntry, SimilarityMetric,
    embeddings::{EmbeddingConfig, providers::create_embedder},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔺 Piramid Embedding Example\n");

    // Configuration - choose your provider
    let use_ollama = std::env::var("USE_OLLAMA").is_ok();
    
    let config = if use_ollama {
        println!("Using Ollama (local embedding)");
        EmbeddingConfig {
            provider: "ollama".to_string(),
            model: "nomic-embed-text".to_string(),
            api_key: None,
            base_url: Some("http://localhost:11434".to_string()),
            options: serde_json::json!({}),
        }
    } else {
        println!("Using OpenAI");
        let api_key = std::env::var("OPENAI_API_KEY")
            .expect("Set OPENAI_API_KEY environment variable");
        
        EmbeddingConfig {
            provider: "openai".to_string(),
            model: "text-embedding-3-small".to_string(),
            api_key: Some(api_key),
            base_url: None,
            options: serde_json::json!({}),
        }
    };

    // Create embedder
    let embedder = create_embedder(&config)?;
    println!("   Provider: {}", embedder.provider_name());
    println!("   Model: {}", embedder.model_name());
    if let Some(dims) = embedder.dimensions() {
        println!("   Dimensions: {}", dims);
    }
    println!();

    // Sample documents
    let documents = vec![
        "The quick brown fox jumps over the lazy dog",
        "A fast red fox leaps above a sleepy canine",
        "Python is a popular programming language",
        "Rust is a systems programming language",
        "Machine learning models can process natural language",
    ];

    // Create storage
    let mut storage = VectorStorage::open("examples_embeddings.db")?;

    println!("📝 Embedding and storing {} documents...", documents.len());
    
    // Embed and store each document
    for (idx, text) in documents.iter().enumerate() {
        let response = embedder.embed(text).await?;
        
        let entry = VectorEntry::with_metadata(
            response.embedding,
            text.to_string(),
            piramid::metadata([
                ("index", piramid::MetadataValue::Integer(idx as i64)),
            ]),
        );
        
        let id = storage.store(entry)?;
        
        if let Some(tokens) = response.tokens {
            println!("   ✓ Doc {}: {} (tokens: {})", idx + 1, id, tokens);
        } else {
            println!("   ✓ Doc {}: {}", idx + 1, id);
        }
    }

    println!("\n🔍 Searching by text query...");
    
    // Search queries
    let queries = vec![
        "animals and foxes",
        "programming languages",
        "artificial intelligence",
    ];

    for query in queries {
        println!("\n   Query: \"{}\"", query);
        
        // Embed the query
        let response = embedder.embed(query).await?;
        
        // Search with the embedded query
        let results = storage.search(&response.embedding, 3, SimilarityMetric::Cosine);
        
        for (rank, result) in results.iter().enumerate() {
            println!("      {}. Score: {:.4} - {}", 
                rank + 1, 
                result.score, 
                result.text
            );
        }
    }

    println!("\n📦 Batch embedding example...");
    
    let batch_texts = vec![
        "The cat sat on the mat".to_string(),
        "The dog played in the park".to_string(),
        "The bird flew in the sky".to_string(),
    ];

    let batch_responses = embedder.embed_batch(&batch_texts).await?;
    println!("   ✓ Embedded {} texts in batch", batch_responses.len());
    
    for response in batch_responses {
        let entry = VectorEntry::new(
            response.embedding,
            batch_texts[0].clone(), // simplified for example
        );
        storage.store(entry)?;
    }

    println!("\n✅ Demo complete!");
    println!("   Storage file: examples_embeddings.db");
    println!("   Total vectors: {}", storage.count());

    Ok(())
}
