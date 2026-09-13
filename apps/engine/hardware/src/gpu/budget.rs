//! Device memory accounting: a budget of usable bytes, divided into pools for model weights, the
//! key/value cache and the index, or shared by all three when no split is set. A [Reservation]
//! holds bytes until it is dropped.

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
    /// Vectors and candidates uploaded for retrieval.
    Index,
}

impl MemoryPool {
    /// Every pool, in reporting order.
    pub const ALL: [MemoryPool; 3] = [MemoryPool::Weights, MemoryPool::KvCache, MemoryPool::Index];

    /// Stable lowercase name.
    pub fn as_str(&self) -> &'static str {
        match self {
            MemoryPool::Weights => "weights",
            MemoryPool::KvCache => "kv_cache",
            MemoryPool::Index => "index",
        }
    }

    fn index(self) -> usize {
        match self {
            MemoryPool::Weights => 0,
            MemoryPool::KvCache => 1,
            MemoryPool::Index => 2,
        }
    }
}

/// Shares of the usable budget for weights, key/value cache and index.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PoolShares {
    /// Share for model weights.
    pub weights: f32,
    /// Share for the key/value cache.
    pub kv_cache: f32,
    /// Share for the index.
    pub index: f32,
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
                let parts = [shares.weights, shares.kv_cache, shares.index];
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

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, reason = "assertions in tests")]

    use super::*;

    const GB: u64 = 1 << 30;

    #[test]
    fn a_split_budget_caps_each_pool_and_a_dropped_reservation_returns_its_bytes() {
        let budget = DeviceBudget::new(
            8 * GB,
            BudgetSettings {
                limit_bytes: Some(5 * GB),
                reserve_bytes: GB,
                shares: Some(PoolShares {
                    weights: 0.5,
                    kv_cache: 0.25,
                    index: 0.25,
                }),
            },
        )
        .unwrap();
        assert_eq!(budget.usable_bytes(), 4 * GB);
        assert_eq!(budget.available(MemoryPool::KvCache), GB);
        let held = budget.reserve(MemoryPool::KvCache, GB / 2).unwrap();
        assert!(budget.reserve(MemoryPool::KvCache, GB).is_err());
        assert_eq!(
            budget.available(MemoryPool::Index),
            GB,
            "pools do not borrow"
        );
        drop(held);
        assert_eq!(budget.available(MemoryPool::KvCache), GB);
    }

    #[test]
    fn a_shared_budget_is_first_come_across_pools() {
        let budget = DeviceBudget::new(
            4 * GB,
            BudgetSettings {
                limit_bytes: None,
                reserve_bytes: 0,
                shares: None,
            },
        )
        .unwrap();
        let _weights = budget.reserve(MemoryPool::Weights, 3 * GB).unwrap();
        assert_eq!(budget.available(MemoryPool::Index), GB);
        assert!(budget.reserve(MemoryPool::KvCache, 2 * GB).is_err());
        let usage = budget.usage();
        assert_eq!(usage[0].used_bytes, 3 * GB);
    }

    #[test]
    fn impossible_settings_are_refused() {
        let reserve = BudgetSettings {
            limit_bytes: Some(GB),
            reserve_bytes: 2 * GB,
            shares: None,
        };
        assert!(DeviceBudget::new(8 * GB, reserve).is_err());
        let shares = BudgetSettings {
            limit_bytes: None,
            reserve_bytes: 0,
            shares: Some(PoolShares {
                weights: 0.7,
                kv_cache: 0.3,
                index: 0.2,
            }),
        };
        assert!(DeviceBudget::new(8 * GB, shares).is_err());
    }

    #[test]
    fn concurrent_reservations_never_exceed_a_pool() {
        const CAPACITY: u64 = 1000;
        for shares in [
            None,
            Some(PoolShares {
                weights: 1.0,
                kv_cache: 0.0,
                index: 0.0,
            }),
        ] {
            let budget = DeviceBudget::new(
                CAPACITY,
                BudgetSettings {
                    limit_bytes: None,
                    reserve_bytes: 0,
                    shares,
                },
            )
            .unwrap();
            let held = std::sync::Mutex::new(Vec::new());
            std::thread::scope(|scope| {
                for _ in 0..8 {
                    scope.spawn(|| {
                        for _ in 0..CAPACITY {
                            if let Ok(reservation) = budget.reserve(MemoryPool::Weights, 1) {
                                held.lock().unwrap().push(reservation);
                            }
                        }
                    });
                }
            });
            let held = held.into_inner().unwrap();
            assert_eq!(held.len() as u64, CAPACITY);
            assert_eq!(budget.available(MemoryPool::Weights), 0);
            assert!(budget.reserve(MemoryPool::Weights, u64::MAX).is_err());
            drop(held);
            assert_eq!(budget.available(MemoryPool::Weights), CAPACITY);
            assert_eq!(budget.usage()[0].used_bytes, 0);
        }
    }
}
