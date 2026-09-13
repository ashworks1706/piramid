//! Page bookkeeping for the attention key/value cache: which slot each token lands in.

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
    /// Bytes one token occupies across every layer's keys and values, saturating at usize::MAX.
    pub fn bytes_per_token(&self) -> usize {
        self.checked_bytes_per_token().unwrap_or(usize::MAX)
    }

    fn checked_bytes_per_token(&self) -> Option<usize> {
        [
            self.layers,
            self.kv_heads,
            self.head_dim,
            self.bytes_per_element,
        ]
        .into_iter()
        .try_fold(2usize, usize::checked_mul)
    }

    /// Whole pages of block_size tokens that fit in budget_bytes.
    pub fn blocks_within(&self, budget_bytes: u64, block_size: usize) -> usize {
        let Some(per_block) = self
            .checked_bytes_per_token()
            .and_then(|bytes| bytes.checked_mul(block_size))
            .and_then(|bytes| u64::try_from(bytes).ok())
        else {
            return 0;
        };
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

/// The content of one published prefix page: the hash of the page before it and its tokens.
#[derive(Debug, Clone, PartialEq, Eq)]
struct PrefixKey {
    parent: u64,
    tokens: Box<[u32]>,
}

impl PrefixKey {
    fn matches(&self, parent: u64, tokens: &[u32]) -> bool {
        self.parent == parent && *self.tokens == *tokens
    }
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
    /// A pool of num_blocks pages of block_size tokens each, capped so every slot fits a u32.
    pub fn new(num_blocks: usize, block_size: usize, prefix_sharing: bool) -> Self {
        let block_size = block_size.max(1);
        let slot_limit = u64::from(u32::MAX) + 1;
        let max_blocks = usize::try_from(slot_limit / block_size as u64).unwrap_or(usize::MAX);
        let total = num_blocks.min(max_blocks);
        Self {
            block_size,
            prefix_sharing,
            refcounts: vec![0; total],
            free: (0..total).rev().map(block_id).collect(),
            evictable: VecDeque::new(),
            by_hash: HashMap::new(),
            keys: vec![None; total],
            stats: PoolStats {
                total_blocks: total,
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

    /// Starts a table for a prompt, sharing cached prefix pages, and returns tokens already cached.
    pub fn start_sequence(&mut self, prompt: &[u32]) -> (BlockTable, usize) {
        let mut table = BlockTable::default();
        if !self.prefix_sharing || prompt.len() < 2 {
            return (table, 0);
        }
        let shareable = (prompt.len() - 1) / self.block_size;
        self.stats.prefix_lookup_tokens += (shareable * self.block_size) as u64;
        let mut parent = 0u64;
        for chunk in prompt.chunks_exact(self.block_size).take(shareable) {
            let hash = hash_prefix(parent, chunk);
            let Some(&block) = self.by_hash.get(&hash) else {
                break;
            };
            let published = self.keys[block as usize]
                .as_ref()
                .is_some_and(|(_, key)| key.matches(parent, chunk));
            if !published {
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
                block_id(block * self.block_size + position % self.block_size)
            })
            .collect()
    }

    /// Makes every full page of a table reusable by later prompts sharing its full token sequence.
    pub fn publish_prefix(&mut self, table: &BlockTable, tokens: &[u32]) {
        if !self.prefix_sharing {
            return;
        }
        let full = table.tokens.min(tokens.len()) / self.block_size;
        let mut parent = 0u64;
        for (index, chunk) in tokens.chunks_exact(self.block_size).take(full).enumerate() {
            let block = table.blocks[index];
            let hash = hash_prefix(parent, chunk);
            match &self.keys[block as usize] {
                Some((existing, _)) if *existing == hash => {}
                Some(_) => return,
                None => {
                    if self.by_hash.contains_key(&hash) {
                        return;
                    }
                    self.by_hash.insert(hash, block);
                    let key = PrefixKey {
                        parent,
                        tokens: chunk.into(),
                    };
                    self.keys[block as usize] = Some((hash, key));
                }
            }
            parent = hash;
        }
    }

    /// Returns every page of a table to the pool; published prefix pages stay reusable until evicted.
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

/// Hash of a prefix page from the hash of the page before it and its tokens.
fn hash_prefix(parent: u64, tokens: &[u32]) -> u64 {
    let mut hasher = DefaultHasher::new();
    parent.hash(&mut hasher);
    tokens.hash(&mut hasher);
    hasher.finish()
}

/// A block or slot index as a u32. The pool size keeps every index in range.
fn block_id(index: usize) -> u32 {
    u32::try_from(index).unwrap_or(u32::MAX)
}
