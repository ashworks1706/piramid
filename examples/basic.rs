use piramid::{VectorEntry, VectorStorage};

fn main() {
    // Open/create a storage file
    let mut storage = VectorStorage::open("my_vectors.db").unwrap();

    // Create some vector entries (in real use, these would be embeddings from an LLM)
    let entry1 = VectorEntry::new(
        vec![0.1, 0.2, 0.3, 0.4],  // embedding vector
        "Hello world".to_string(),  // original text
    );

    let entry2 = VectorEntry::new(
        vec![0.5, 0.6, 0.7, 0.8],
        "Rust is awesome".to_string(),
    );

    // Store them
    let id1 = storage.store(entry1).unwrap();
    let id2 = storage.store(entry2).unwrap();

    println!("Stored entry 1 with ID: {}", id1);
    println!("Stored entry 2 with ID: {}", id2);

    // Retrieve by ID
    if let Some(entry) = storage.get(&id1) {
        println!("\nRetrieved: '{}' with vector {:?}", entry.text, entry.vector);
    }

    // Get all entries
    println!("\nAll stored vectors ({} total):", storage.count());
    for entry in storage.get_all() {
        println!("  - {}: '{}'", entry.id, entry.text);
    }
}
