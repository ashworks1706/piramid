//! The resident vector slab every search reads through.

use std::collections::HashMap;

use piramid_core::error::{Result, ServerError, StorageError};
use uuid::Uuid;

use crate::storage::vectors::{VectorReader, VectorSlab};

/// Every vector of a collection, resident in memory as one contiguous buffer.
///
/// Rows are one flat float buffer at a fixed stride, addressed through a Uuid to u32 ordinal map.
/// Ordinals are stable: a removed row becomes a hole, and the next insert reuses it.
#[derive(Default)]
pub struct VectorStore {
    /// Row-major, dim floats per row. Holes are still allocated and their contents are stale.
    slab: Vec<f32>,
    /// Row width, fixed by the first vector stored.
    dim: Option<usize>,
    /// Id to row.
    ordinals: HashMap<Uuid, u32>,
    /// Row to id, in row order. The entry for a hole is stale; ordinals holds what is live.
    ids: Vec<Uuid>,
    /// Whether each row is live, in row order.
    live: Vec<bool>,
    /// Holes, reused before the slab grows.
    free: Vec<u32>,
}

impl VectorStore {
    /// An empty store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Row width, once anything has been stored.
    pub fn dim(&self) -> Option<usize> {
        self.dim
    }

    /// Rows allocated but not live. [VectorReader::as_slab] marks each of them as a hole.
    pub fn holes(&self) -> usize {
        self.free.len()
    }

    /// Insert or replace the vector for id.
    ///
    /// A width other than the stride of the store is an error.
    pub fn put(&mut self, id: Uuid, vector: &[f32]) -> Result<()> {
        let dim = *self.dim.get_or_insert(vector.len());
        if vector.len() != dim {
            return Err(ServerError::InvalidRequest(format!(
                "Vector dimension mismatch: collection holds {dim}, got {}",
                vector.len()
            ))
            .into());
        }
        let ordinal = match self.ordinals.get(&id) {
            Some(existing) => *existing,
            None => self.claim_row(id, dim)?,
        };
        let start = ordinal as usize * dim;
        self.slab[start..start + dim].copy_from_slice(vector);
        Ok(())
    }

    /// Remove the vector for id, leaving its row as a hole.
    pub fn remove(&mut self, id: &Uuid) {
        let Some(ordinal) = self.ordinals.remove(id) else {
            return;
        };
        self.live[ordinal as usize] = false;
        self.free.push(ordinal);
    }

    /// Resident bytes: the slab, the two id maps and the liveness of each row.
    pub fn usage_bytes(&self) -> usize {
        self.slab.len() * std::mem::size_of::<f32>()
            + self.ordinals.len() * (std::mem::size_of::<Uuid>() + std::mem::size_of::<u32>())
            + self.ids.len() * std::mem::size_of::<Uuid>()
            + self.live.len() * std::mem::size_of::<bool>()
    }

    /// A hole if there is one, otherwise a new row at the end.
    fn claim_row(&mut self, id: Uuid, dim: usize) -> Result<u32> {
        let ordinal = match self.free.pop() {
            Some(ordinal) => {
                self.ids[ordinal as usize] = id;
                self.live[ordinal as usize] = true;
                ordinal
            }
            None => {
                let ordinal = ordinal_for_row(self.ids.len())?;
                self.ids.push(id);
                self.live.push(true);
                self.slab.resize(self.slab.len() + dim, 0.0);
                ordinal
            }
        };
        self.ordinals.insert(id, ordinal);
        Ok(ordinal)
    }

    fn row(&self, ordinal: u32, dim: usize) -> &[f32] {
        let start = ordinal as usize * dim;
        &self.slab[start..start + dim]
    }
}

/// The ordinal of the row at index, or an error when it does not fit in a u32.
pub fn ordinal_for_row(index: usize) -> Result<u32> {
    u32::try_from(index)
        .map_err(|_| StorageError::StorageFull("vector store exceeds u32::MAX rows".into()).into())
}

impl VectorReader for VectorStore {
    fn get(&self, id: &Uuid) -> Option<&[f32]> {
        let dim = self.dim?;
        self.ordinals.get(id).map(|ordinal| self.row(*ordinal, dim))
    }

    fn iter<'a>(&'a self) -> Box<dyn Iterator<Item = (Uuid, &'a [f32])> + 'a> {
        let Some(dim) = self.dim else {
            return Box::new(std::iter::empty());
        };
        Box::new(
            self.ordinals
                .iter()
                .map(move |(id, ordinal)| (*id, self.row(*ordinal, dim))),
        )
    }

    fn len(&self) -> usize {
        self.ordinals.len()
    }

    fn dim(&self) -> Option<usize> {
        self.dim
    }

    /// The whole slab, holes included and marked, once a vector has been stored.
    fn as_slab(&self) -> Option<VectorSlab<'_>> {
        let dim = self.dim?;
        Some(VectorSlab {
            data: self.slab.as_slice(),
            dim,
            ids: self.ids.as_slice(),
            live: self.live.as_slice(),
        })
    }
}
