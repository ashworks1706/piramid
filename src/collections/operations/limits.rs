use super::super::collection::Collection;
use crate::error::{Result, ServerError};

pub(super) fn enforce_single(storage: &Collection, entry_bytes: usize) -> Result<()> {
    let limits = storage.config.limits;

    if let Some(max_vecs) = limits.max_vectors {
        if storage.count() >= max_vecs {
            return Err(
                ServerError::InvalidRequest("Collection max vectors reached".into()).into(),
            );
        }
    }

    if let Some(max_bytes) = limits.max_bytes {
        let current_size = storage.record_store.used_bytes();
        let required = current_size.saturating_add(entry_bytes as u64);
        if required > max_bytes {
            return Err(ServerError::InvalidRequest("Collection max size reached".into()).into());
        }
    }

    if let Some(max_vec_bytes) = limits.max_vector_bytes {
        if entry_bytes > max_vec_bytes {
            return Err(
                ServerError::InvalidRequest("Vector exceeds max allowed size".into()).into(),
            );
        }
    }

    Ok(())
}

pub(super) fn enforce_batch(
    storage: &Collection,
    total_entries: usize,
    total_bytes: u64,
    max_entry_bytes: Option<usize>,
) -> Result<()> {
    let limits = storage.config.limits;

    if let Some(max_vecs) = limits.max_vectors {
        let current = storage.count();
        if current.saturating_add(total_entries) > max_vecs {
            return Err(
                ServerError::InvalidRequest("Collection max vectors reached".into()).into(),
            );
        }
    }

    if let Some(max_bytes) = limits.max_bytes {
        let current_size = storage.record_store.used_bytes();
        let required = current_size.saturating_add(total_bytes);
        if required > max_bytes {
            return Err(ServerError::InvalidRequest("Collection max size reached".into()).into());
        }
    }

    if let Some(max_vec_bytes) = max_entry_bytes {
        if max_vec_bytes > 0 {
            if let Some(cfg_limit) = limits.max_vector_bytes {
                if max_vec_bytes > cfg_limit {
                    return Err(ServerError::InvalidRequest(
                        "Vector exceeds max allowed size".into(),
                    )
                    .into());
                }
            }
        }
    }

    Ok(())
}
