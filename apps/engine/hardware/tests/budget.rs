//! Device memory budgets: pool capacities, shared and split accounting, and reservations.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "assertions in tests"
)]

use piramid_hardware::gpu::{BudgetSettings, DeviceBudget, MemoryPool, PoolShares};

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
                vectors: 0.25,
            }),
        },
    )
    .unwrap();
    assert_eq!(budget.usable_bytes(), 4 * GB);
    assert_eq!(budget.available(MemoryPool::KvCache), GB);
    let held = budget.reserve(MemoryPool::KvCache, GB / 2).unwrap();
    assert!(budget.reserve(MemoryPool::KvCache, GB).is_err());
    assert_eq!(
        budget.available(MemoryPool::Vectors),
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
    assert_eq!(budget.available(MemoryPool::Vectors), GB);
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
            vectors: 0.2,
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
            vectors: 0.0,
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
