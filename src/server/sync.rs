use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard, PoisonError};
use std::time::{Duration, Instant};
use crate::error::{Result, ServerError};

// Helper trait for RwLock error handling with timeout support
// 
// Provides convenience methods to convert lock poisoning errors
// into our error type system, and prevent deadlocks with timeouts.
pub trait LockHelper<T> {
    fn read(&self, timeout: Duration) -> Result<RwLockReadGuard<'_, T>>;
    fn write(&self, timeout: Duration) -> Result<RwLockWriteGuard<'_, T>>;
}

impl<T> LockHelper<T> for RwLock<T> {
    fn read(&self, timeout: Duration) -> Result<RwLockReadGuard<'_, T>> {
        let deadline = Instant::now() + timeout;
        loop {
            match self.try_read() {
                Ok(guard) => return Ok(guard),
                Err(_) if Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(_) => return Err(ServerError::Timeout.into()),
            }
        }
    }

    fn write(&self, timeout: Duration) -> Result<RwLockWriteGuard<'_, T>> {
        let deadline = Instant::now() + timeout;
        loop {
            match self.try_write() {
                Ok(guard) => return Ok(guard),
                Err(_) if Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(_) => return Err(ServerError::Timeout.into()),
            }
        }
    }

}
