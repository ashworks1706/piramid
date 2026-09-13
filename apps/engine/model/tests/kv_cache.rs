//! The paged key/value cache allocator and its sizing.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "assertions in tests"
)]

use piramid_model::inference::kv_cache::{BlockAllocator, KvLayout};

#[test]
fn slots_walk_pages_in_token_order() {
    let mut pool = BlockAllocator::new(4, 2, false);
    let (mut table, cached) = pool.start_sequence(&[1, 2, 3]);
    assert_eq!(cached, 0);
    pool.reserve(&mut table, 3).unwrap();
    let slots = pool.advance(&mut table, 3).unwrap();
    let blocks = table.blocks().to_vec();
    assert_eq!(slots, vec![blocks[0] * 2, blocks[0] * 2 + 1, blocks[1] * 2]);
    assert_eq!(table.len(), 3);
}

#[test]
fn writing_past_the_reserved_pages_is_an_error() {
    let mut pool = BlockAllocator::new(4, 2, false);
    let (mut table, _) = pool.start_sequence(&[1]);
    pool.reserve(&mut table, 2).unwrap();
    assert!(pool.advance(&mut table, 3).is_err());
}

#[test]
fn an_exhausted_pool_refuses_rather_than_overcommits() {
    let mut pool = BlockAllocator::new(2, 4, false);
    let (mut first, _) = pool.start_sequence(&[1]);
    pool.reserve(&mut first, 8).unwrap();
    let (mut second, _) = pool.start_sequence(&[1]);
    assert!(pool.reserve(&mut second, 1).is_err());
    pool.release(first);
    pool.reserve(&mut second, 8).unwrap();
}

#[test]
fn a_published_prefix_is_shared_but_never_the_last_token() {
    let mut pool = BlockAllocator::new(8, 2, true);
    let prompt = [5, 6, 7, 8, 9];
    let (mut first, _) = pool.start_sequence(&prompt);
    pool.reserve(&mut first, prompt.len()).unwrap();
    pool.advance(&mut first, prompt.len()).unwrap();
    pool.publish_prefix(&first, &prompt);

    let (second, cached) = pool.start_sequence(&prompt);
    assert_eq!(cached, 4);
    assert_eq!(second.blocks(), &first.blocks()[..2]);

    let (_, cached) = pool.start_sequence(&[5, 6, 7, 8]);
    assert_eq!(cached, 2, "the last token of a prompt is always computed");

    let (_, cached) = pool.start_sequence(&[5, 6, 0, 8, 9]);
    assert_eq!(cached, 2, "sharing stops at the first page that differs");
    assert!(pool.stats().prefix_hit_tokens >= 8);
}

#[test]
fn a_released_prefix_stays_reusable_until_evicted() {
    let mut pool = BlockAllocator::new(2, 2, true);
    let prompt = [1, 2, 3];
    let (mut first, _) = pool.start_sequence(&prompt);
    pool.reserve(&mut first, 3).unwrap();
    pool.advance(&mut first, 3).unwrap();
    pool.publish_prefix(&first, &prompt);
    pool.release(first);
    assert_eq!(pool.stats().cached_blocks, 1);

    let (table, cached) = pool.start_sequence(&prompt);
    assert_eq!(cached, 2);
    pool.release(table);

    let (mut other, _) = pool.start_sequence(&[9]);
    pool.reserve(&mut other, 4).unwrap();
    assert_eq!(pool.stats().evictions, 1);
    let (_, cached) = pool.start_sequence(&prompt);
    assert_eq!(cached, 0);
}

#[test]
fn the_layout_sizes_the_pool_from_a_byte_budget() {
    let layout = KvLayout {
        layers: 24,
        kv_heads: 2,
        head_dim: 64,
        bytes_per_element: 2,
    };
    assert_eq!(layout.bytes_per_token(), 12_288);
    assert_eq!(layout.blocks_within(12_288 * 16 * 10, 16), 10);
}

#[test]
fn every_slot_of_a_large_pool_is_a_distinct_u32() {
    let block_size = 1 << 20;
    let pool = BlockAllocator::new(1 << 16, block_size, false);
    assert_eq!(pool.stats().total_blocks, 1 << 12);
    assert_eq!(pool.total_slots() as u64, u64::from(u32::MAX) + 1);

    let mut pool = pool;
    let (mut table, _) = pool.start_sequence(&[1]);
    pool.reserve(&mut table, pool.total_slots()).unwrap();
    let last = pool.slots(&table, pool.total_slots() - 1, 1);
    assert_eq!(table.blocks().last(), Some(&((1 << 12) - 1)));
    assert_eq!(last, vec![u32::MAX]);
}

#[test]
fn an_oversized_layout_fits_no_blocks() {
    let layout = KvLayout {
        layers: usize::MAX,
        kv_heads: 2,
        head_dim: 64,
        bytes_per_element: 2,
    };
    assert_eq!(layout.blocks_within(u64::MAX, 16), 0);
}
