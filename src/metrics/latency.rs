// Operation latency tracking for metrics

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct LatencyTracker {
    // Moving average of operation latencies (in microseconds)
    insert_latency_us: Arc<AtomicU64>,
    search_latency_us: Arc<AtomicU64>,
    delete_latency_us: Arc<AtomicU64>,
    update_latency_us: Arc<AtomicU64>,
    
    // Operation counts
    insert_count: Arc<AtomicU64>,
    search_count: Arc<AtomicU64>,
    delete_count: Arc<AtomicU64>,
    update_count: Arc<AtomicU64>,
}

impl Default for LatencyTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl LatencyTracker {
    pub fn new() -> Self {
        Self {
            insert_latency_us: Arc::new(AtomicU64::new(0)),
            search_latency_us: Arc::new(AtomicU64::new(0)),
            delete_latency_us: Arc::new(AtomicU64::new(0)),
            update_latency_us: Arc::new(AtomicU64::new(0)),
            insert_count: Arc::new(AtomicU64::new(0)),
            search_count: Arc::new(AtomicU64::new(0)),
            delete_count: Arc::new(AtomicU64::new(0)),
            update_count: Arc::new(AtomicU64::new(0)),
        }
    }
    
    // Record insert operation latency
    pub fn record_insert(&self, duration: Duration) {
        self.insert_count.fetch_add(1, Ordering::Relaxed);
        let us = duration.as_micros() as u64;
        self.update_moving_average(&self.insert_latency_us, us, &self.insert_count);
    }
    
    // Record search operation latency
    pub fn record_search(&self, duration: Duration) {
        self.search_count.fetch_add(1, Ordering::Relaxed);
        let us = duration.as_micros() as u64;
        self.update_moving_average(&self.search_latency_us, us, &self.search_count);
    }
    
    // Record delete operation latency
    pub fn record_delete(&self, duration: Duration) {
        self.delete_count.fetch_add(1, Ordering::Relaxed);
        let us = duration.as_micros() as u64;
        self.update_moving_average(&self.delete_latency_us, us, &self.delete_count);
    }
    
    // Record update operation latency
    pub fn record_update(&self, duration: Duration) {
        self.update_count.fetch_add(1, Ordering::Relaxed);
        let us = duration.as_micros() as u64;
        self.update_moving_average(&self.update_latency_us, us, &self.update_count);
    }
    
    // Get average insert latency in milliseconds
    pub fn avg_insert_latency_ms(&self) -> Option<f32> {
        let us = self.insert_latency_us.load(Ordering::Relaxed);
        if us > 0 {
            Some(us as f32 / 1000.0)
        } else {
            None
        }
    }
    
    // Get average search latency in milliseconds
    pub fn avg_search_latency_ms(&self) -> Option<f32> {
        let us = self.search_latency_us.load(Ordering::Relaxed);
        if us > 0 {
            Some(us as f32 / 1000.0)
        } else {
            None
        }
    }
    
    // Get average delete latency in milliseconds
    pub fn avg_delete_latency_ms(&self) -> Option<f32> {
        let us = self.delete_latency_us.load(Ordering::Relaxed);
        if us > 0 {
            Some(us as f32 / 1000.0)
        } else {
            None
        }
    }
    
    // Get average update latency in milliseconds
    pub fn avg_update_latency_ms(&self) -> Option<f32> {
        let us = self.update_latency_us.load(Ordering::Relaxed);
        if us > 0 {
            Some(us as f32 / 1000.0)
        } else {
            None
        }
    }
    
    // Update moving average (exponential moving average with alpha=0.2)
    fn update_moving_average(&self, avg: &AtomicU64, new_value: u64, count: &AtomicU64) {
        let current = avg.load(Ordering::Relaxed);
        let cnt = count.load(Ordering::Relaxed);
        
        // For first few samples, use simple average
        if cnt <= 5 {
            let new_avg = ((current * (cnt - 1)) + new_value) / cnt;
            avg.store(new_avg, Ordering::Relaxed);
        } else {
            // Exponential moving average: new_avg = 0.8 * old_avg + 0.2 * new_value
            let new_avg = ((current * 4) + new_value) / 5;
            avg.store(new_avg, Ordering::Relaxed);
        }
    }
}

// Helper to time an operation and return result + duration
pub async fn time_operation<F, T>(operation: F) -> (T, Duration)
where
    F: std::future::Future<Output = T>,
{
    let start = Instant::now();
    let result = operation.await;
    let duration = start.elapsed();
    (result, duration)
}

// Helper for sync operations
pub fn time_operation_sync<F, T>(operation: F) -> (T, Duration)
where
    F: FnOnce() -> T,
{
    let start = Instant::now();
    let result = operation();
    let duration = start.elapsed();
    (result, duration)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_latency_tracking() {
        let tracker = LatencyTracker::new();
        
        // Initially no latencies
        assert_eq!(tracker.avg_insert_latency_ms(), None);
        assert_eq!(tracker.avg_search_latency_ms(), None);
        
        // Record some operations
        tracker.record_insert(Duration::from_millis(10));
        tracker.record_insert(Duration::from_millis(20));
        tracker.record_search(Duration::from_millis(5));
        
        // Should have averages now
        assert!(tracker.avg_insert_latency_ms().is_some());
        assert!(tracker.avg_search_latency_ms().is_some());
        
        // Insert average should be around 15ms
        let insert_avg = tracker.avg_insert_latency_ms().unwrap();
        assert!(insert_avg > 10.0 && insert_avg < 20.0);
        
        // Search should be around 5ms
        let search_avg = tracker.avg_search_latency_ms().unwrap();
        assert!(search_avg > 4.0 && search_avg < 6.0);
    }

    #[tokio::test]
    async fn test_time_operation() {
        let (result, duration) = time_operation(async {
            tokio::time::sleep(Duration::from_millis(10)).await;
            42
        }).await;
        
        assert_eq!(result, 42);
        assert!(duration.as_millis() >= 10);
    }

    #[test]
    fn test_time_operation_sync() {
        let (result, duration) = time_operation_sync(|| {
            thread::sleep(Duration::from_millis(10));
            "hello"
        });
        
        assert_eq!(result, "hello");
        assert!(duration.as_millis() >= 10);
    }
}
