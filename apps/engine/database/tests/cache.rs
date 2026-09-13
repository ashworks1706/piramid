#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "assertions in tests"
)]
//! The resident vector store and the cache manager that wraps it.

use piramid_core::config::CacheConfig;
use piramid_database::cache::ordinal_for_row;
use piramid_database::storage::VectorReader;
use piramid_database::{CacheManager, VectorStore};
use uuid::Uuid;

/// A store holding the given rows in order.
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
fn a_row_past_the_u32_range_is_refused() {
    assert_eq!(ordinal_for_row(u32::MAX as usize).unwrap(), u32::MAX);
    let error = ordinal_for_row(u32::MAX as usize + 1).unwrap_err();
    assert!(error.to_string().contains("u32::MAX rows"), "{error}");
}

#[test]
fn an_empty_store_offers_no_slab_rather_than_an_empty_one() {
    let store = VectorStore::new();

    assert!(store.as_slab().is_none());
    assert!(store.is_empty());
}

#[test]
fn the_wrapper_forwards_every_reader_method_to_the_store() {
    let mut cache = CacheManager::new(CacheConfig::default());
    let id = Uuid::new_v4();
    cache.put_vector(id, &[1.0, 2.0]).unwrap();
    cache.put_vector(Uuid::new_v4(), &[3.0, 4.0]).unwrap();

    assert_eq!(VectorReader::len(&cache), 2);
    assert_eq!(VectorReader::dim(&cache), Some(2));
    assert_eq!(cache.get(&id), Some([1.0, 2.0].as_slice()));

    let slab = cache.as_slab().expect("the store underneath is contiguous");
    assert_eq!(slab.dim, 2);
    assert_eq!(slab.data.len(), 4);
    assert_eq!(slab.rows(), 2);

    let mut out = [0.0; 2];
    cache.gather_into(&[id], &mut out).unwrap();
    assert_eq!(out, [1.0, 2.0]);
}
