// Parallelism configuration for concurrent operations

use serde::{Deserialize, Serialize};

// Parallelism mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParallelismMode {
    // Single-threaded execution
    SingleThreaded,
    // Use all available CPU cores
    Auto,
    // Use a specific number of threads
    Fixed(usize),
}

impl Default for ParallelismMode {
    fn default() -> Self {
        ParallelismMode::Auto
    }
}

// Parallelism configuration
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ParallelismConfig {
    // Thread pool mode
    pub mode: ParallelismMode,
    
    // Enable parallel search (when applicable)
    pub parallel_search: bool,
}

impl Default for ParallelismConfig {
    fn default() -> Self {
        ParallelismConfig {
            mode: ParallelismMode::Auto,
            parallel_search: true,
        }
    }
}

impl ParallelismConfig {
    // Single-threaded mode (for debugging or low-resource environments)
    pub fn single_threaded() -> Self {
        ParallelismConfig {
            mode: ParallelismMode::SingleThreaded,
            parallel_search: false,
        }
    }
    
    // Use a fixed number of threads
    pub fn fixed(num_threads: usize) -> Self {
        ParallelismConfig {
            mode: ParallelismMode::Fixed(num_threads),
            parallel_search: true,
        }
    }
    
    // Get the number of threads to use
    pub fn num_threads(&self) -> usize {
        match self.mode {
            ParallelismMode::SingleThreaded => 1,
            ParallelismMode::Auto => num_cpus::get(),
            ParallelismMode::Fixed(n) => n,
        }
    }

    pub fn with_num_threads(mut self, n: usize) -> Self {
        self.mode = ParallelismMode::Fixed(n);
        self.parallel_search = n > 1;
        self
    }
}
