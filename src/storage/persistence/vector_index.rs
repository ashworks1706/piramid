// Saves and loads index structures to disk

use std::fs;
use std::path::Path;
use std::io::{Read, BufReader};
use crate::error::Result;
use crate::index::{SerializableIndex, VectorIndex};

// Get the index file path for a collection
pub fn get_index_file_path(collection_path: &str) -> String {
    format!("{}.vecindex.db", collection_path)
}

// Save any index to disk
pub fn save_vector_index(collection_path: &str, index: &dyn VectorIndex) -> Result<()> {
    let serializable = index.to_serializable();
    
    let bytes = bincode::serialize(&serializable)?;
    let index_path = get_index_file_path(collection_path);
    fs::write(index_path, bytes)?;
    Ok(())
}


pub fn warm_file(path: &str) -> Result<()> {
    let file = match fs::File::open(path) {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e.into()),
    };
    let mut reader = BufReader::new(file);
    let mut buf = vec![0u8; 4 * 1024 * 1024]; // 4MB window to fault pages
    loop {
        let read = reader.read(&mut buf)?;
        if read == 0 {
            break;
        }
        std::hint::black_box(&buf[..read]);
    }
    Ok(())
}
// Load index from disk
pub fn load_vector_index(collection_path: &str) -> Result<Option<Box<dyn VectorIndex>>> {
    // construct the expected file path for the index based on the collection path. 
    // If the file exists, we read the bytes from the file and deserialize them into a SerializableIndex enum. convert the SerializableIndex into a Box<dyn VectorIndex> trait object and return it wrapped in Some. 
    // If the file does not exist, we return Ok(None) to indicate that there is no existing index to load.
    let index_path = get_index_file_path(collection_path);
    
    if !Path::new(&index_path).exists() {
        return Ok(None);
    }
    
    let bytes = fs::read(index_path)?;
    let serializable: SerializableIndex = bincode::deserialize(&bytes)?;
    Ok(Some(serializable.to_trait_object()))
}
