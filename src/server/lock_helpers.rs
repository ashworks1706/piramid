use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard, PoisonError};
use axum::http::StatusCode;

pub trait LockHelper<T> {
    fn read_or_err(&self) -> Result<RwLockReadGuard<'_, T>, (StatusCode, String)>;
    fn write_or_err(&self) -> Result<RwLockWriteGuard<'_, T>, (StatusCode, String)>;
}

impl<T> LockHelper<T> for RwLock<T> {
    fn read_or_err(&self) -> Result<RwLockReadGuard<'_, T>, (StatusCode, String)> {
        self.read()
            .map_err(|e: PoisonError<_>| {
                (StatusCode::INTERNAL_SERVER_ERROR, format!("Lock poisoned: {}", e))
            })
    }

    fn write_or_err(&self) -> Result<RwLockWriteGuard<'_, T>, (StatusCode, String)> {
        self.write()
            .map_err(|e: PoisonError<_>| {
                (StatusCode::INTERNAL_SERVER_ERROR, format!("Lock poisoned: {}", e))
            })
    }
}
