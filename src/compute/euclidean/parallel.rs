// Parallel implementation of Euclidean distance
// Uses rayon for multi-threaded computation

use rayon::prelude::*;

pub fn euclidean_distance_parallel(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len(), "Vectors must have same length");

    let chunk_size = (a.len() / num_cpus::get()).max(1024);

    let sum_sq: f32 = a
        .par_chunks(chunk_size)
        .zip(b.par_chunks(chunk_size))
        .map(|(chunk_a, chunk_b)| {
            let mut sum = 0.0;
            for i in 0..chunk_a.len() {
                let diff = chunk_a[i] - chunk_b[i];
                sum += diff * diff;
            }
            sum
        })
        .sum();

    sum_sq.sqrt()
}

pub fn euclidean_distance_squared_parallel(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len(), "Vectors must have same length");

    let chunk_size = (a.len() / num_cpus::get()).max(1024);

    a.par_chunks(chunk_size)
        .zip(b.par_chunks(chunk_size))
        .map(|(chunk_a, chunk_b)| {
            let mut sum = 0.0;
            for i in 0..chunk_a.len() {
                let diff = chunk_a[i] - chunk_b[i];
                sum += diff * diff;
            }
            sum
        })
        .sum()
}
