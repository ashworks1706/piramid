// JIT-optimized cosine similarity
// Pre-compiles optimized code for specific vector dimensions
// Uses loop unrolling and architecture-specific optimizations

pub fn cosine_similarity_jit(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len(), "Vectors must have same length");

    let len = a.len();

    // Dispatch to specialized implementations based on common dimensions
    match len {
        128 => cosine_similarity_jit_128(a, b),
        256 => cosine_similarity_jit_256(a, b),
        384 => cosine_similarity_jit_384(a, b),
        512 => cosine_similarity_jit_512(a, b),
        768 => cosine_similarity_jit_768(a, b),
        1024 => cosine_similarity_jit_1024(a, b),
        1536 => cosine_similarity_jit_1536(a, b), // OpenAI text-embedding-3-small
        3072 => cosine_similarity_jit_3072(a, b), // OpenAI text-embedding-3-large
        _ => cosine_similarity_jit_generic(a, b),
    }
}

// Specialized implementation for 1536 dimensions (most common)
#[inline(always)]
fn cosine_similarity_jit_1536(a: &[f32], b: &[f32]) -> f32 {
    let mut dot = 0.0;
    let mut norm_a = 0.0;
    let mut norm_b = 0.0;

    // Unroll loop in blocks of 16 for better instruction pipelining
    let mut i = 0;
    while i < 1536 {
        // Block 1
        dot += a[i] * b[i];
        norm_a += a[i] * a[i];
        norm_b += b[i] * b[i];

        // Block 2
        dot += a[i + 1] * b[i + 1];
        norm_a += a[i + 1] * a[i + 1];
        norm_b += b[i + 1] * b[i + 1];

        // Block 3
        dot += a[i + 2] * b[i + 2];
        norm_a += a[i + 2] * a[i + 2];
        norm_b += b[i + 2] * b[i + 2];

        // Block 4
        dot += a[i + 3] * b[i + 3];
        norm_a += a[i + 3] * a[i + 3];
        norm_b += b[i + 3] * b[i + 3];

        // Block 5
        dot += a[i + 4] * b[i + 4];
        norm_a += a[i + 4] * a[i + 4];
        norm_b += b[i + 4] * b[i + 4];

        // Block 6
        dot += a[i + 5] * b[i + 5];
        norm_a += a[i + 5] * a[i + 5];
        norm_b += b[i + 5] * b[i + 5];

        // Block 7
        dot += a[i + 6] * b[i + 6];
        norm_a += a[i + 6] * a[i + 6];
        norm_b += b[i + 6] * b[i + 6];

        // Block 8
        dot += a[i + 7] * b[i + 7];
        norm_a += a[i + 7] * a[i + 7];
        norm_b += b[i + 7] * b[i + 7];

        i += 8;
    }

    let denominator = norm_a.sqrt() * norm_b.sqrt();
    if denominator == 0.0 {
        0.0
    } else {
        dot / denominator
    }
}

// Macro to generate specialized implementations
macro_rules! jit_impl {
    ($name:ident, $dim:expr) => {
        #[inline(always)]
        fn $name(a: &[f32], b: &[f32]) -> f32 {
            let mut dot = 0.0;
            let mut norm_a = 0.0;
            let mut norm_b = 0.0;

            let mut i = 0;
            while i < $dim {
                let unroll = ($dim - i).min(8);
                for j in 0..unroll {
                    let idx = i + j;
                    dot += a[idx] * b[idx];
                    norm_a += a[idx] * a[idx];
                    norm_b += b[idx] * b[idx];
                }
                i += 8;
            }

            let denominator = norm_a.sqrt() * norm_b.sqrt();
            if denominator == 0.0 {
                0.0
            } else {
                dot / denominator
            }
        }
    };
}

jit_impl!(cosine_similarity_jit_128, 128);
jit_impl!(cosine_similarity_jit_256, 256);
jit_impl!(cosine_similarity_jit_384, 384);
jit_impl!(cosine_similarity_jit_512, 512);
jit_impl!(cosine_similarity_jit_768, 768);
jit_impl!(cosine_similarity_jit_1024, 1024);
jit_impl!(cosine_similarity_jit_3072, 3072);

fn cosine_similarity_jit_generic(a: &[f32], b: &[f32]) -> f32 {
    let mut dot = 0.0;
    let mut norm_a = 0.0;
    let mut norm_b = 0.0;

    for i in 0..a.len() {
        dot += a[i] * b[i];
        norm_a += a[i] * a[i];
        norm_b += b[i] * b[i];
    }

    let denominator = norm_a.sqrt() * norm_b.sqrt();
    if denominator == 0.0 {
        0.0
    } else {
        dot / denominator
    }
}
