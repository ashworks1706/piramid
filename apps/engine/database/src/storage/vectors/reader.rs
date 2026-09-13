//! Vector access abstraction: how indexes read vectors they do not own.

use std::collections::HashMap;

use uuid::Uuid;

/// Every vector as one contiguous buffer, with the id of each row.
///
/// The data is row-major at dim floats per row, and entry i of ids names row i.
pub struct VectorSlab<'a> {
    /// Row-major floats, rows() * dim long.
    pub data: &'a [f32],
    /// Row width.
    pub dim: usize,
    /// Id of each row, in row order.
    pub ids: &'a [Uuid],
}

impl VectorSlab<'_> {
    /// Number of rows.
    pub fn rows(&self) -> usize {
        self.ids.len()
    }
}

/// Read-only access to the vectors of a collection.
pub trait VectorReader: Sync {
    /// Vector for an id, if present.
    fn get(&self, id: &Uuid) -> Option<&[f32]>;

    /// Iterate every id and vector pair.
    fn iter<'a>(&'a self) -> Box<dyn Iterator<Item = (Uuid, &'a [f32])> + 'a>;

    /// Number of vectors available.
    fn len(&self) -> usize;

    /// Whether no vectors are available.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Row width, if the reader holds a uniform-width set.
    fn dim(&self) -> Option<usize> {
        self.iter().next().map(|(_, vector)| vector.len())
    }

    /// The whole vector set as one contiguous row-major buffer, if it is stored that way.
    ///
    /// A reader over scattered allocations returns None.
    ///
    /// A wrapper forwarding this trait forwards this method too.
    fn as_slab(&self) -> Option<VectorSlab<'_>> {
        None
    }

    /// Copies the vectors for ids into out, row-major. out is ids.len() * dim long.
    fn gather_into(&self, ids: &[Uuid], out: &mut [f32]) -> Option<()> {
        let dim = self.dim()?;
        if out.len() != ids.len() * dim {
            return None;
        }
        for (id, slot) in ids.iter().zip(out.chunks_exact_mut(dim)) {
            slot.copy_from_slice(self.get(id)?);
        }
        Some(())
    }
}

/// A [VectorReader] over a scattered HashMap of owned vectors.
pub struct HashMapVectorReader<'a> {
    vectors: &'a HashMap<Uuid, Vec<f32>>,
}

impl<'a> HashMapVectorReader<'a> {
    /// Borrow a map of vectors as a reader.
    pub fn new(vectors: &'a HashMap<Uuid, Vec<f32>>) -> Self {
        Self { vectors }
    }
}

impl VectorReader for HashMapVectorReader<'_> {
    fn get(&self, id: &Uuid) -> Option<&[f32]> {
        self.vectors.get(id).map(Vec::as_slice)
    }

    fn iter<'a>(&'a self) -> Box<dyn Iterator<Item = (Uuid, &'a [f32])> + 'a> {
        Box::new(
            self.vectors
                .iter()
                .map(|(id, vector)| (*id, vector.as_slice())),
        )
    }

    fn len(&self) -> usize {
        self.vectors.len()
    }
}
