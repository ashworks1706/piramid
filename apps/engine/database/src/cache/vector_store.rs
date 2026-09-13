//! The resident vector slab every index reads through.

use std::collections::HashMap;

use piramid_core::error::{Result, ServerError};
use uuid::Uuid;

use crate::storage::vectors::{VectorReader, VectorSlab};

/// Every vector of a collection, resident in memory as one contiguous buffer.
///
/// The ANN indexes resolve ids here, so an evicted entry is a search failure.
///
/// Rows are one flat float buffer at a fixed stride, addressed through a Uuid to u32 ordinal map.
/// Ordinals are stable: a removed row becomes a hole, and the next insert reuses it.
#[derive(Default)]
pub struct VectorStore {
    /// Row-major, dim floats per row. Holes are still allocated and their contents are stale.
    slab: Vec<f32>,
    /// Row width, fixed by the first vector stored and cleared only by [VectorStore::clear].
    dim: Option<usize>,
    /// Id to row.
    ordinals: HashMap<Uuid, u32>,
    /// Row to id, in row order. The entry for a hole is stale; ordinals holds what is live.
    ids: Vec<Uuid>,
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

    /// Rows allocated but not live. Any of them makes [VectorReader::as_slab] return None.
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
            None => self.claim_row(id, dim),
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
        self.free.push(ordinal);
    }

    /// Drop every vector and the row width. Only correct before a rebuild repopulates the store.
    pub fn clear(&mut self) {
        self.slab.clear();
        self.ids.clear();
        self.free.clear();
        self.ordinals.clear();
        self.dim = None;
    }

    /// Resident bytes: the slab plus the two id maps.
    pub fn usage_bytes(&self) -> usize {
        self.slab.len() * std::mem::size_of::<f32>()
            + self.ordinals.len() * (std::mem::size_of::<Uuid>() + std::mem::size_of::<u32>())
            + self.ids.len() * std::mem::size_of::<Uuid>()
    }

    /// A hole if there is one, otherwise a new row at the end.
    fn claim_row(&mut self, id: Uuid, dim: usize) -> u32 {
        let ordinal = match self.free.pop() {
            Some(ordinal) => {
                self.ids[ordinal as usize] = id;
                ordinal
            }
            None => {
                let ordinal = u32::try_from(self.ids.len()).unwrap_or(u32::MAX);
                self.ids.push(id);
                self.slab.resize(self.slab.len() + dim, 0.0);
                ordinal
            }
        };
        self.ordinals.insert(id, ordinal);
        ordinal
    }

    fn row(&self, ordinal: u32) -> &[f32] {
        let dim = self.dim.unwrap_or(0);
        let start = ordinal as usize * dim;
        &self.slab[start..start + dim]
    }
}

impl VectorReader for VectorStore {
    fn get(&self, id: &Uuid) -> Option<&[f32]> {
        self.ordinals.get(id).map(|ordinal| self.row(*ordinal))
    }

    fn iter<'a>(&'a self) -> Box<dyn Iterator<Item = (Uuid, &'a [f32])> + 'a> {
        Box::new(
            self.ordinals
                .iter()
                .map(|(id, ordinal)| (*id, self.row(*ordinal))),
        )
    }

    fn len(&self) -> usize {
        self.ordinals.len()
    }

    fn dim(&self) -> Option<usize> {
        self.dim
    }

    /// The whole slab, when every allocated row is live.
    ///
    /// A store with holes returns None.
    fn as_slab(&self) -> Option<VectorSlab<'_>> {
        let dim = self.dim?;
        self.free.is_empty().then_some(VectorSlab {
            data: self.slab.as_slice(),
            dim,
            ids: self.ids.as_slice(),
        })
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    reason = "a failed assertion is the point of a test"
)]
mod tests {
    use super::*;

    fn store(rows: &[(Uuid, [f32; 2])]) -> VectorStore {
        let mut store = VectorStore::new();
        for (id, vector) in rows {
            store.put(*id, vector).unwrap();
        }
        store
    }

    #[test]
    fn rows_are_contiguous_and_readable_by_id() {
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let store = store(&[(a, [1.0, 2.0]), (b, [3.0, 4.0])]);

        assert_eq!(store.get(&a), Some([1.0, 2.0].as_slice()));
        assert_eq!(store.get(&b), Some([3.0, 4.0].as_slice()));
        assert_eq!(store.len(), 2);
        assert_eq!(VectorReader::dim(&store), Some(2));

        let slab = store.as_slab().unwrap();
        assert_eq!(slab.dim, 2);
        assert_eq!(slab.data, [1.0, 2.0, 3.0, 4.0]);
        assert_eq!(slab.ids, [a, b]);
        assert_eq!(slab.rows(), 2);
    }

    #[test]
    fn a_replaced_vector_is_written_in_place_rather_than_appended() {
        let a = Uuid::new_v4();
        let mut store = store(&[(a, [1.0, 2.0])]);

        store.put(a, &[9.0, 9.0]).unwrap();

        assert_eq!(store.len(), 1);
        assert_eq!(store.as_slab().unwrap().data, [9.0, 9.0]);
    }

    #[test]
    fn a_width_that_is_not_the_stride_is_refused() {
        let mut store = store(&[(Uuid::new_v4(), [1.0, 2.0])]);

        let error = store.put(Uuid::new_v4(), &[1.0, 2.0, 3.0]).unwrap_err();

        assert!(error.to_string().contains("dimension mismatch"), "{error}");
        assert_eq!(store.len(), 1);
    }

    #[test]
    fn a_hole_withdraws_the_slab_and_the_next_insert_fills_it() {
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let mut store = store(&[(a, [1.0, 2.0]), (b, [3.0, 4.0])]);

        store.remove(&a);

        assert_eq!(store.len(), 1);
        assert_eq!(store.get(&a), None);
        assert_eq!(store.holes(), 1);
        // A hole withdraws the slab.
        assert!(store.as_slab().is_none());
        // The ordinal of b did not move.
        assert_eq!(store.get(&b), Some([3.0, 4.0].as_slice()));

        let c = Uuid::new_v4();
        store.put(c, &[5.0, 6.0]).unwrap();

        assert_eq!(store.holes(), 0);
        let slab = store.as_slab().unwrap();
        assert_eq!(slab.data, [5.0, 6.0, 3.0, 4.0]);
        assert_eq!(slab.ids, [c, b]);
        assert_eq!(store.get(&c), Some([5.0, 6.0].as_slice()));
    }

    #[test]
    fn iteration_and_gathering_see_only_live_rows() {
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let c = Uuid::new_v4();
        let mut store = store(&[(a, [1.0, 1.0]), (b, [2.0, 2.0]), (c, [3.0, 3.0])]);
        store.remove(&b);

        let mut seen: Vec<Uuid> = store.iter().map(|(id, _)| id).collect();
        seen.sort();
        let mut expected = vec![a, c];
        expected.sort();
        assert_eq!(seen, expected);

        let mut out = [0.0; 4];
        store.gather_into(&[c, a], &mut out).unwrap();
        assert_eq!(out, [3.0, 3.0, 1.0, 1.0]);
    }

    #[test]
    fn clearing_releases_the_stride_so_a_rebuild_can_change_it() {
        let mut store = store(&[(Uuid::new_v4(), [1.0, 2.0])]);

        store.clear();

        assert_eq!(store.len(), 0);
        assert_eq!(VectorReader::dim(&store), None);
        assert!(store.as_slab().is_none());

        // Clearing allows a new width.
        store.put(Uuid::new_v4(), &[1.0, 2.0, 3.0]).unwrap();
        assert_eq!(VectorReader::dim(&store), Some(3));
    }

    #[test]
    fn an_empty_store_offers_no_slab_rather_than_an_empty_one() {
        let store = VectorStore::new();

        assert!(store.as_slab().is_none());
        assert!(store.is_empty());
    }
}
