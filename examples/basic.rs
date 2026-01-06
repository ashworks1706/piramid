use piramid::{
    VectorEntry, VectorStorage, DistanceMetric, 
    Filter, metadata, SearchResult
};

fn main() {
    // Clean up any previous test data
    let _ = std::fs::remove_file("demo_vectors.db");

    // Open/create a storage file
    let mut storage = VectorStorage::open("demo_vectors.db").unwrap();

    println!("=== Piramid Vector Database Demo ===\n");

    // Create some vector entries with metadata
    // In real use, these vectors would be embeddings from an LLM
    let entries = vec![
        VectorEntry::with_metadata(
            vec![0.9, 0.1, 0.0, 0.0],
            "Rust is a systems programming language".to_string(),
            metadata([
                ("category", "programming".into()),
                ("language", "rust".into()),
                ("year", 2015i64.into()),
            ]),
        ),
        VectorEntry::with_metadata(
            vec![0.8, 0.2, 0.1, 0.0],
            "Python is great for machine learning".to_string(),
            metadata([
                ("category", "programming".into()),
                ("language", "python".into()),
                ("year", 1991i64.into()),
            ]),
        ),
        VectorEntry::with_metadata(
            vec![0.1, 0.9, 0.0, 0.0],
            "The quick brown fox jumps over the lazy dog".to_string(),
            metadata([
                ("category", "text".into()),
                ("type", "pangram".into()),
            ]),
        ),
        VectorEntry::with_metadata(
            vec![0.85, 0.15, 0.05, 0.0],
            "JavaScript runs in the browser".to_string(),
            metadata([
                ("category", "programming".into()),
                ("language", "javascript".into()),
                ("year", 1995i64.into()),
            ]),
        ),
        VectorEntry::with_metadata(
            vec![0.7, 0.3, 0.0, 0.0],
            "Go is designed for concurrency".to_string(),
            metadata([
                ("category", "programming".into()),
                ("language", "go".into()),
                ("year", 2009i64.into()),
            ]),
        ),
    ];

    // Store all entries
    println!("Storing {} vectors...\n", entries.len());
    for entry in entries {
        let id = storage.store(entry).unwrap();
        println!("  Stored: {}", id);
    }

    // --- Similarity Search ---
    println!("\n=== Similarity Search ===");
    
    // Search with a query vector similar to "programming" topics
    let query = vec![0.85, 0.15, 0.0, 0.0];
    println!("\nQuery vector: {:?}", query);
    println!("Finding top 3 similar vectors (Cosine):\n");

    let results = storage.search(&query, 3, DistanceMetric::Cosine);
    print_results(&results);

    // --- Filtered Search ---
    println!("\n=== Filtered Search ===");
    println!("Finding programming languages released after 2000:\n");

    let filter = Filter::new()
        .eq("category", "programming")
        .gt("year", 2000i64);

    let results = storage.search_with_filter(&query, 10, DistanceMetric::Cosine, Some(&filter));
    print_results(&results);

    // --- Different Distance Metrics ---
    println!("\n=== Distance Metrics Comparison ===");
    println!("Same query with different metrics:\n");

    for metric in [DistanceMetric::Cosine, DistanceMetric::Euclidean, DistanceMetric::DotProduct] {
        println!("{:?}:", metric);
        let results = storage.search(&query, 2, metric);
        for r in &results {
            println!("  {:.4} - {}", r.score, r.text);
        }
        println!();
    }

    // --- Delete ---
    println!("=== Delete Operation ===");
    let first_id = storage.get_all()[0].id;
    println!("Deleting vector {}...", first_id);
    storage.delete(&first_id).unwrap();
    println!("Remaining vectors: {}\n", storage.count());

    // Clean up
    std::fs::remove_file("demo_vectors.db").unwrap();
    println!("✓ Demo complete!");
}

fn print_results(results: &[SearchResult]) {
    for (i, result) in results.iter().enumerate() {
        println!(
            "  {}. [score: {:.4}] {}",
            i + 1,
            result.score,
            result.text
        );
        if !result.metadata.is_empty() {
            println!("     metadata: {:?}", result.metadata);
        }
    }
}
