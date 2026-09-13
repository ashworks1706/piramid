//! Attention key/value cache bookkeeping: device memory is a pool of fixed-size pages, each
//! sequence holds a [BlockTable] naming its pages in order, and full pages with identical
//! prefixes are shared between sequences. The backend holds the page storage; this module decides
//! which slot each token is written to.

use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, VecDeque};
use std::hash::{Hash, Hasher};

use piramid_core::error::InferenceError;

/// Shape of the cache for one model, used to size the page pool.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KvLayout {
    /// Decoder layers, each holding a key and a value store.
    pub layers: usize,
    /// Key/value heads per layer.
    pub kv_heads: usize,
    /// Width of one head.
    pub head_dim: usize,
    /// Bytes per stored element.
    pub bytes_per_element: usize,
}

impl KvLayout {
    /// Bytes one token occupies across every layer's keys and values.
    pub fn bytes_per_token(&self) -> usize {
        2 * self.layers * self.kv_heads * self.head_dim * self.bytes_per_element
    }

    /// Whole pages of block_size tokens that fit in budget_bytes.
    pub fn blocks_within(&self, budget_bytes: u64, block_size: usize) -> usize {
        let per_block = (self.bytes_per_token() * block_size) as u64;
        if per_block == 0 {
            return 0;
        }
        usize::try_from(budget_bytes / per_block).unwrap_or(usize::MAX)
    }
}

/// The pages one sequence holds, in token order, and how many tokens are written into them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BlockTable {
    blocks: Vec<u32>,
    tokens: usize,
}

impl BlockTable {
    /// Page identifiers in token order.
    pub fn blocks(&self) -> &[u32] {
        &self.blocks
    }

    /// Tokens whose keys and values are in the cache.
    pub fn len(&self) -> usize {
        self.tokens
    }

    /// Whether the table holds no tokens.
    pub fn is_empty(&self) -> bool {
        self.tokens == 0
    }
}

/// Usage counters for the page pool.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PoolStats {
    /// Pages in the pool.
    pub total_blocks: usize,
    /// Pages held by at least one sequence.
    pub used_blocks: usize,
    /// Pages no sequence holds that still carry a reusable prefix.
    pub cached_blocks: usize,
    /// Prefix pages evicted to make room.
    pub evictions: u64,
    /// Prompt tokens served from shared pages.
    pub prefix_hit_tokens: u64,
    /// Prompt tokens looked up for sharing.
    pub prefix_lookup_tokens: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PrefixKey {
    parent: u64,
    tokens: Vec<u32>,
}

/// Allocates pages to sequences and shares full prefix pages between them.
#[derive(Debug)]
pub struct BlockAllocator {
    block_size: usize,
    prefix_sharing: bool,
    refcounts: Vec<u32>,
    free: Vec<u32>,
    evictable: VecDeque<u32>,
    by_hash: HashMap<u64, u32>,
    keys: Vec<Option<(u64, PrefixKey)>>,
    stats: PoolStats,
}

impl BlockAllocator {
    /// A pool of num_blocks pages of block_size tokens each.
    pub fn new(num_blocks: usize, block_size: usize, prefix_sharing: bool) -> Self {
        let total = u32::try_from(num_blocks).unwrap_or(u32::MAX);
        Self {
            block_size: block_size.max(1),
            prefix_sharing,
            refcounts: vec![0; total as usize],
            free: (0..total).rev().collect(),
            evictable: VecDeque::new(),
            by_hash: HashMap::new(),
            keys: vec![None; total as usize],
            stats: PoolStats {
                total_blocks: total as usize,
                ..PoolStats::default()
            },
        }
    }

    /// Tokens per page.
    pub fn block_size(&self) -> usize {
        self.block_size
    }

    /// Slots in the pool: pages times tokens per page.
    pub fn total_slots(&self) -> usize {
        self.refcounts.len() * self.block_size
    }

    /// Pages a new allocation can take, counting cached prefix pages that can be evicted.
    pub fn available_blocks(&self) -> usize {
        self.free.len() + self.evictable.len()
    }

    /// Pages needed to hold tokens.
    pub fn blocks_for(&self, tokens: usize) -> usize {
        tokens.div_ceil(self.block_size)
    }

    /// Current usage counters.
    pub fn stats(&self) -> PoolStats {
        PoolStats {
            used_blocks: self.refcounts.iter().filter(|&&count| count > 0).count(),
            cached_blocks: self.evictable.len(),
            ..self.stats
        }
    }

    /// Start a table for a prompt, sharing every full prefix page already cached.
    ///
    /// Returns the table and how many leading tokens are already in the cache. The last prompt
    /// token is never shared, so at least one token is always computed.
    pub fn start_sequence(&mut self, prompt: &[u32]) -> (BlockTable, usize) {
        let mut table = BlockTable::default();
        if !self.prefix_sharing || prompt.len() < 2 {
            return (table, 0);
        }
        let shareable = (prompt.len() - 1) / self.block_size;
        self.stats.prefix_lookup_tokens += (shareable * self.block_size) as u64;
        let mut parent = 0u64;
        for chunk in prompt.chunks_exact(self.block_size).take(shareable) {
            let key = PrefixKey {
                parent,
                tokens: chunk.to_vec(),
            };
            let hash = hash_key(&key);
            let Some(&block) = self.by_hash.get(&hash) else {
                break;
            };
            if self.keys[block as usize].as_ref().map(|(_, k)| k) != Some(&key) {
                break;
            }
            self.retain(block);
            table.blocks.push(block);
            table.tokens += self.block_size;
            parent = hash;
        }
        self.stats.prefix_hit_tokens += table.tokens as u64;
        let cached = table.tokens;
        (table, cached)
    }

    /// Grow a table so it can hold total_tokens, taking pages from the pool.
    pub fn reserve(
        &mut self,
        table: &mut BlockTable,
        total_tokens: usize,
    ) -> Result<(), InferenceError> {
        let needed = self
            .blocks_for(total_tokens)
            .saturating_sub(table.blocks.len());
        if needed > self.available_blocks() {
            return Err(InferenceError::Runtime(format!(
                "kv cache has {} free pages, {needed} needed",
                self.available_blocks()
            )));
        }
        for _ in 0..needed {
            let block = self.take_block()?;
            table.blocks.push(block);
        }
        Ok(())
    }

    /// Record that the next count tokens of a table are written, and return their slots.
    pub fn advance(
        &mut self,
        table: &mut BlockTable,
        count: usize,
    ) -> Result<Vec<u32>, InferenceError> {
        let end = table.tokens + count;
        if self.blocks_for(end) > table.blocks.len() {
            return Err(InferenceError::Runtime(format!(
                "table holds {} pages, {end} tokens need {}",
                table.blocks.len(),
                self.blocks_for(end)
            )));
        }
        let slots = self.slots(table, table.tokens, count);
        table.tokens = end;
        Ok(slots)
    }

    /// Slots of count tokens of a table starting at position start.
    pub fn slots(&self, table: &BlockTable, start: usize, count: usize) -> Vec<u32> {
        (start..start + count)
            .map(|position| {
                let block = table.blocks[position / self.block_size] as usize;
                (block * self.block_size + position % self.block_size) as u32
            })
            .collect()
    }

    /// Make every full page of a table reusable by later prompts with the same prefix.
    ///
    /// tokens is the full token sequence the table holds.
    pub fn publish_prefix(&mut self, table: &BlockTable, tokens: &[u32]) {
        if !self.prefix_sharing {
            return;
        }
        let full = table.tokens.min(tokens.len()) / self.block_size;
        let mut parent = 0u64;
        for (index, chunk) in tokens.chunks_exact(self.block_size).take(full).enumerate() {
            let block = table.blocks[index];
            let key = PrefixKey {
                parent,
                tokens: chunk.to_vec(),
            };
            let hash = hash_key(&key);
            match &self.keys[block as usize] {
                Some((existing, _)) if *existing == hash => {}
                Some(_) => return,
                None => {
                    if self.by_hash.contains_key(&hash) {
                        return;
                    }
                    self.by_hash.insert(hash, block);
                    self.keys[block as usize] = Some((hash, key));
                }
            }
            parent = hash;
        }
    }

    /// Return every page of a table to the pool. Pages carrying a published prefix stay
    /// reusable until a new allocation evicts them.
    pub fn release(&mut self, table: BlockTable) {
        for block in table.blocks.into_iter().rev() {
            let count = &mut self.refcounts[block as usize];
            *count = count.saturating_sub(1);
            if *count == 0 {
                if self.keys[block as usize].is_some() {
                    self.evictable.push_back(block);
                } else {
                    self.free.push(block);
                }
            }
        }
    }

    fn retain(&mut self, block: u32) {
        if self.refcounts[block as usize] == 0 {
            self.evictable.retain(|&cached| cached != block);
        }
        self.refcounts[block as usize] += 1;
    }

    fn take_block(&mut self) -> Result<u32, InferenceError> {
        let block = if let Some(block) = self.free.pop() {
            block
        } else if let Some(block) = self.evictable.pop_front() {
            if let Some((hash, _)) = self.keys[block as usize].take() {
                self.by_hash.remove(&hash);
            }
            self.stats.evictions += 1;
            block
        } else {
            return Err(InferenceError::Runtime(
                "kv cache has no free pages".to_string(),
            ));
        };
        self.refcounts[block as usize] = 1;
        Ok(block)
    }
}

fn hash_key(key: &PrefixKey) -> u64 {
    let mut hasher = DefaultHasher::new();
    key.hash(&mut hasher);
    hasher.finish()
}

impl Hash for PrefixKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.parent.hash(state);
        self.tokens.hash(state);
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, reason = "assertions in tests")]

    use super::*;

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
}
