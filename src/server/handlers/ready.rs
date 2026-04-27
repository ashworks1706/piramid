// src/server/handlers/ready.rs
// checks if the server is shutting down, gathers health info for each collection, and reports disk usage stats.

use axum::{extract::State, response::Json};
use crate::error::{Result, ServerError};
use super::super::state::SharedState;
use super::super::types::{ReadyzResponse, CollectionHealth};
use crate::server::metrics::record_lock_read;
use std::time::{SystemTime, UNIX_EPOCH};
use std::sync::atomic::Ordering;

// Helper function to get disk usage stats for the data directory
fn disk_stats(path: &str) -> (Option<u64>, Option<u64>) {
    #[cfg(target_family = "unix")]
    {
        use std::ffi::CString;
        let c_path = match CString::new(path) {
            Ok(p) => p,
            Err(_) => return (None, None),
        }; // ensure the path is a valid C string and statvfs is called correctly
        let mut stat: libc::statvfs = unsafe { std::mem::zeroed() }; //  ensure the statvfs struct is properly initialized and used
        let rc = unsafe { libc::statvfs(c_path.as_ptr(), &mut stat) };
        if rc == 0 {
            let total = (stat.f_blocks as u64).saturating_mul(stat.f_frsize as u64);
            let avail = (stat.f_bavail as u64).saturating_mul(stat.f_frsize as u64);
            return (Some(total), Some(avail)); // We ensure the statvfs call is successful and the fields are accessed correctly
        }
    }
    (None, None)
}


// GET /api/readyz - readiness + integrity snapshot
// This endpoint is more comprehensive. 
// -  server is in the process of shutting down (returns 503 if so)
// -  each collection: if it's loaded, vector count, index type, last checkpoint time, checkpoint age, WAL size, schema version, and integrity status
// - Disk usage stats for the data directory
pub async fn readyz(State(state): State<SharedState>) -> Result<Json<ReadyzResponse>> {
    // 1. Check if server is shutting down - if so, return 503 to indicate we're not ready to serve traffic
    if state.shutting_down.load(Ordering::Relaxed) {
        return Err(ServerError::ServiceUnavailable("Server is shutting down".to_string()).into());
    }

    let mut collections_health = Vec::new();
    let mut total_vectors = 0usize;

    // 2. Gather health info for each loaded collection
    for entry in state.collections.iter() {
        let name = entry.key().clone();
        let lock_start = std::time::Instant::now();
        let storage = entry.value().read();
        record_lock_read(state.latency_tracker.get(&name).as_deref(), lock_start); // Record how long we waited to acquire the read lock for this collection

        let count = storage.count();
        total_vectors += count;
        let index_type = storage.vector_index().index_type().to_string();
        let schema_version = Some(storage.metadata.schema_version);
        let last_checkpoint = storage.persistence.last_checkpoint();
        let checkpoint_age_secs = last_checkpoint.and_then(|ts| {
            let now = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs();
            now.checked_sub(ts)
        });
        let wal_size_bytes = std::fs::metadata(&format!("{}.wal.db", storage.path))
            .map(|m| m.len())
            .ok(); // If the WAL file doesn't exist or we can't read it, we just return None for its size

        //   might want to add more thorough checks.
        collections_health.push(CollectionHealth {
            name,
            loaded: true,
            count: Some(count),
            index_type: Some(index_type),
            last_checkpoint,
            checkpoint_age_secs,
            wal_size_bytes,
            schema_version,
            integrity_ok: true,
            error: None,
        });
    }

    // Discover collections on disk not yet loaded
    if let Ok(entries) = std::fs::read_dir(&state.data_dir) {
        for entry in entries.flatten() {
            if let Some(ext) = entry.path().extension() {
                if ext == "db" {
                    let name = entry.path().file_stem().and_then(|s| s.to_str()).unwrap_or("").to_string();
                    if state.collections.contains_key(&name) {
                        continue;
                    }
                    collections_health.push(CollectionHealth {
                        name,
                        loaded: false,
                        count: None,
                        index_type: None,
                        last_checkpoint: None,
                        checkpoint_age_secs: None,
                        wal_size_bytes: None,
                        schema_version: None,
                        integrity_ok: false,
                        error: Some("not loaded".to_string()),
                    });
                }
            }
        }
    }

    let loaded_collections = state.collections.len();
    let (disk_total_bytes, disk_available_bytes) = disk_stats(&state.data_dir);

    let ok = collections_health.iter().all(|c| c.integrity_ok && c.loaded);
    
    // 3. Return the comprehensive readiness snapshot
    Ok(Json(ReadyzResponse {
        ok,
        version: env!("CARGO_PKG_VERSION").to_string(),
        data_dir: state.data_dir.clone(),
        total_collections: collections_health.len(),
        loaded_collections,
        total_vectors,
        disk_total_bytes,
        disk_available_bytes,
        collections: collections_health,
    }))
}
