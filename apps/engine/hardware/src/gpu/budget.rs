//! Device memory budget split into pools for weights, kv cache and vectors, or shared by all three.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use crate::gpu::error::{GpuError, GpuResult};

/// What device memory is being claimed for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryPool {
    /// Model weights.
    Weights,
    /// The attention key/value cache.
    KvCache,
    /// Stored vectors and scoring buffers uploaded for search.
    Vectors,
}

impl MemoryPool {
    /// Every pool, in reporting order.
    pub const ALL: [MemoryPool; 3] = [
        MemoryPool::Weights,
        MemoryPool::KvCache,
        MemoryPool::Vectors,
    ];

    /// Stable lowercase name.
    pub fn as_str(&self) -> &'static str {
        match self {
            MemoryPool::Weights => "weights",
            MemoryPool::KvCache => "kv_cache",
            MemoryPool::Vectors => "vectors",
        }
    }

    fn index(self) -> usize {
        match self {
            MemoryPool::Weights => 0,
            MemoryPool::KvCache => 1,
            MemoryPool::Vectors => 2,
        }
    }
}

/// Shares of the usable budget for weights, key/value cache and stored vectors.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PoolShares {
    /// Share for model weights.
    pub weights: f32,
    /// Share for the key/value cache.
    pub kv_cache: f32,
    /// Share for stored vectors and scoring buffers.
    pub vectors: f32,
}

/// How much device memory the process may use and how it is divided.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BudgetSettings {
    /// Upper bound on bytes to claim. None is the whole device.
    pub limit_bytes: Option<u64>,
    /// Bytes held back from the budget for fragmentation and library workspaces.
    pub reserve_bytes: u64,
    /// Division between pools. None lets every pool draw from one shared budget.
    pub shares: Option<PoolShares>,
}

/// One pool's capacity and use at a moment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PoolUsage {
    /// The pool.
    pub pool: MemoryPool,
    /// Bytes the pool may hold. Under a shared budget, the whole budget.
    pub capacity_bytes: u64,
    /// Bytes reserved in the pool.
    pub used_bytes: u64,
}

#[derive(Debug)]
struct Accounts {
    usable: u64,
    shared: bool,
    capacity: [u64; 3],
    used: [AtomicU64; 3],
    total_used: AtomicU64,
}

/// The device memory budget of one device, shared by everything that allocates on it.
#[derive(Debug, Clone)]
pub struct DeviceBudget {
    accounts: Arc<Accounts>,
}

/// Bytes held in a pool, returned when dropped.
#[derive(Debug)]
#[must_use = "the bytes are returned as soon as the reservation is dropped"]
pub struct Reservation {
    accounts: Arc<Accounts>,
    pool: MemoryPool,
    bytes: u64,
}

impl Reservation {
    /// Bytes held.
    pub fn bytes(&self) -> u64 {
        self.bytes
    }

    /// The pool the bytes are held in.
    pub fn pool(&self) -> MemoryPool {
        self.pool
    }
}

impl Drop for Reservation {
    fn drop(&mut self) {
        self.accounts.used[self.pool.index()].fetch_sub(self.bytes, Ordering::Relaxed);
        self.accounts
            .total_used
            .fetch_sub(self.bytes, Ordering::Relaxed);
    }
}

impl DeviceBudget {
    /// A budget over a device holding total_bytes.
    pub fn new(total_bytes: u64, settings: BudgetSettings) -> GpuResult<Self> {
        let ceiling = settings
            .limit_bytes
            .map_or(total_bytes, |limit| limit.min(total_bytes));
        let usable = ceiling.checked_sub(settings.reserve_bytes).ok_or_else(|| {
            GpuError::Allocation(format!(
                "reserve of {} bytes exceeds the {ceiling} byte budget",
                settings.reserve_bytes
            ))
        })?;
        let (shared, capacity) = match settings.shares {
            None => (true, [usable; 3]),
            Some(shares) => {
                let parts = [shares.weights, shares.kv_cache, shares.vectors];
                if parts.iter().any(|share| !(0.0..=1.0).contains(share))
                    || parts.iter().sum::<f32>() > 1.0 + 1e-6
                {
                    return Err(GpuError::Allocation(format!(
                        "pool shares {parts:?} must each be within 0 and 1 and sum to at most 1"
                    )));
                }
                (
                    false,
                    parts.map(|share| (usable as f64 * f64::from(share)) as u64),
                )
            }
        };
        Ok(Self {
            accounts: Arc::new(Accounts {
                usable,
                shared,
                capacity,
                used: [AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0)],
                total_used: AtomicU64::new(0),
            }),
        })
    }

    /// Bytes the budget covers after the reserve.
    pub fn usable_bytes(&self) -> u64 {
        self.accounts.usable
    }

    /// Whether every pool draws from one shared budget.
    pub fn is_shared(&self) -> bool {
        self.accounts.shared
    }

    /// Bytes a pool can still take.
    pub fn available(&self, pool: MemoryPool) -> u64 {
        let accounts = &self.accounts;
        if accounts.shared {
            accounts
                .usable
                .saturating_sub(accounts.total_used.load(Ordering::Relaxed))
        } else {
            accounts.capacity[pool.index()]
                .saturating_sub(accounts.used[pool.index()].load(Ordering::Relaxed))
        }
    }

    /// Hold bytes in a pool, or refuse when the pool cannot take them.
    pub fn reserve(&self, pool: MemoryPool, bytes: u64) -> GpuResult<Reservation> {
        let accounts = &self.accounts;
        let (counter, limit) = if accounts.shared {
            (&accounts.total_used, accounts.usable)
        } else {
            (
                &accounts.used[pool.index()],
                accounts.capacity[pool.index()],
            )
        };
        counter
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
                current.checked_add(bytes).filter(|next| *next <= limit)
            })
            .map_err(|current| {
                GpuError::Allocation(format!(
                    "{bytes} bytes requested from the {} pool with {} available",
                    pool.as_str(),
                    limit.saturating_sub(current)
                ))
            })?;
        if accounts.shared {
            accounts.used[pool.index()].fetch_add(bytes, Ordering::Relaxed);
        } else {
            accounts.total_used.fetch_add(bytes, Ordering::Relaxed);
        }
        Ok(Reservation {
            accounts: Arc::clone(accounts),
            pool,
            bytes,
        })
    }

    /// Capacity and use of every pool.
    pub fn usage(&self) -> Vec<PoolUsage> {
        MemoryPool::ALL
            .iter()
            .map(|&pool| PoolUsage {
                pool,
                capacity_bytes: self.accounts.capacity[pool.index()],
                used_bytes: self.accounts.used[pool.index()].load(Ordering::Relaxed),
            })
            .collect()
    }
}
