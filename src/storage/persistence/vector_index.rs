// Index persistence for all index types (HNSW, IVF, Flat)
// Saves and loads index structures to disk

use std::fs;
use std::path::Path;
use std::io::{Read, BufReader};
use crate::error::Result;
use crate::index::{SerializableIndex, VectorIndex, HnswIndex, IvfIndex, FlatIndex};

// Get the index file path for a collection
pub fn get_index_file_path(collection_path: &str) -> String {
    format!("{}.vecindex.db", collection_path)
}

// Save any index to disk
pub fn save_vector_index(collection_path: &str, index: &dyn VectorIndex) -> Result<()> {
    // Create the appropriate SerializableIndex variant
    let serializable = match index.index_type() {
        crate::index::IndexType::Hnsw => {
            // Downcast to concrete type
            let hnsw_ptr = index as *const dyn VectorIndex as *const HnswIndex;
            // this means we are assuming the caller is passing the correct type of index based on
            // index_type() - this is a bit unsafe but allows us to avoid adding a method to get a
            // serializable version of the index without modifying the trait    
            // In practice, we should ensure that the caller is responsible for passing the correct
            // type of index based on index_type() to avoid undefined behavior
            let hnsw_ref = unsafe { &*hnsw_ptr };
            SerializableIndex::Hnsw(hnsw_ref.clone())
        }
        crate::index::IndexType::Ivf => {
            let ivf_ptr = index as *const dyn VectorIndex as *const IvfIndex;
            let ivf_ref = unsafe { &*ivf_ptr };
            SerializableIndex::Ivf(ivf_ref.clone())
        }
        crate::index::IndexType::Flat => {
            let flat_ptr = index as *const dyn VectorIndex as *const FlatIndex;
            let flat_ref = unsafe { &*flat_ptr };
            SerializableIndex::Flat(flat_ref.clone())
        }
    };
    
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
    // To load an index from disk, we first construct the expected file path for the index based on the collection path. We check if the file exists, and if it does, we read the bytes from the file and deserialize them into a SerializableIndex enum. Finally, we convert the SerializableIndex into a Box<dyn VectorIndex> trait object and return it wrapped in Some. If the file does not exist, we return Ok(None) to indicate that there is no existing index to load.
    let index_path = get_index_file_path(collection_path);
    
    if !Path::new(&index_path).exists() {
        return Ok(None);
    }
    
    let bytes = fs::read(index_path)?;
    let serializable: SerializableIndex = bincode::deserialize(&bytes)?;
    Ok(Some(serializable.to_trait_object()))
}
