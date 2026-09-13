use super::Collection;
use piramid_core::error::{Result, ServerError};

/// Refuse one entry of entry_bytes that would pass a limit. replacing is true when the entry takes
/// the place of a stored one, which does not grow the vector count.
pub(crate) fn enforce_single(
    collection: &Collection,
    entry_bytes: usize,
    replacing: bool,
) -> Result<()> {
    let limits = collection.config.limits;

    if let (Some(max_vecs), false) = (limits.max_vectors, replacing) {
        if collection.count() >= max_vecs {
            return Err(
                ServerError::InvalidRequest("Collection max vectors reached".into()).into(),
            );
        }
    }

    if let Some(max_bytes) = limits.max_bytes {
        let current_size = collection.record_store.used_bytes();
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

pub(crate) fn enforce_batch(
    collection: &Collection,
    total_entries: usize,
    total_bytes: u64,
    max_entry_bytes: Option<usize>,
) -> Result<()> {
    let limits = collection.config.limits;

    if let Some(max_vecs) = limits.max_vectors {
        let current = collection.count();
        if current.saturating_add(total_entries) > max_vecs {
            return Err(
                ServerError::InvalidRequest("Collection max vectors reached".into()).into(),
            );
        }
    }

    if let Some(max_bytes) = limits.max_bytes {
        let current_size = collection.record_store.used_bytes();
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
