// Batched distance kernels over a row-major candidate slab, and a chunked top-k selection.

extern "C" {

__global__ void cosine_rows(const float* query, const float* slab, float* out, unsigned int dim,
                 unsigned int rows, float query_norm_sq) {
    unsigned int row = blockIdx.x * blockDim.x + threadIdx.x;
    if (row >= rows) {
        return;
    }
    const float* candidate = slab + (unsigned long long)row * dim;
    float dot = 0.0f;
    float norm = 0.0f;
    for (unsigned int i = 0; i < dim; ++i) {
        dot += query[i] * candidate[i];
        norm += candidate[i] * candidate[i];
    }
    float denominator = sqrt(query_norm_sq) * sqrt(norm);
    out[row] = denominator == 0.0f ? 0.0f : dot / denominator;
}

__global__ void dot_rows(const float* query, const float* slab, float* out, unsigned int dim,
              unsigned int rows) {
    unsigned int row = blockIdx.x * blockDim.x + threadIdx.x;
    if (row >= rows) {
        return;
    }
    const float* candidate = slab + (unsigned long long)row * dim;
    float dot = 0.0f;
    for (unsigned int i = 0; i < dim; ++i) {
        dot += query[i] * candidate[i];
    }
    out[row] = dot;
}

__global__ void euclidean_rows(const float* query, const float* slab, float* out, unsigned int dim,
                    unsigned int rows) {
    unsigned int row = blockIdx.x * blockDim.x + threadIdx.x;
    if (row >= rows) {
        return;
    }
    const float* candidate = slab + (unsigned long long)row * dim;
    float sum = 0.0f;
    for (unsigned int i = 0; i < dim; ++i) {
        float diff = query[i] - candidate[i];
        sum += diff * diff;
    }
    out[row] = sqrt(sum);
}

// One thread per chunk of the input. Each writes the k highest scores of its chunk, in
// descending order, to out_scores and out_index at offset chunk_id * k. Slots past the end of a
// short chunk hold the lowest finite float and index 0xFFFFFFFF. When has_index is zero the position in
// scores is the index; otherwise in_index maps positions to indices.
__global__ void select_top_k(const float* scores, const unsigned int* in_index, unsigned int has_index,
                  unsigned int n, unsigned int chunk, unsigned int k, float* out_scores,
                  unsigned int* out_index) {
    unsigned int chunk_id = blockIdx.x * blockDim.x + threadIdx.x;
    unsigned long long start = (unsigned long long)chunk_id * chunk;
    if (start >= n) {
        return;
    }
    unsigned long long end = start + chunk;
    if (end > n) {
        end = n;
    }
    float* best = out_scores + (unsigned long long)chunk_id * k;
    unsigned int* best_index = out_index + (unsigned long long)chunk_id * k;
    for (unsigned int j = 0; j < k; ++j) {
        best[j] = -3.4028234663852886e38f;
        best_index[j] = 0xFFFFFFFFu;
    }
    unsigned int filled = 0;
    for (unsigned long long p = start; p < end; ++p) {
        float score = scores[p];
        if (score != score || (has_index && in_index[p] == 0xFFFFFFFFu)) {
            continue;
        }
        if (filled == k && score <= best[k - 1]) {
            continue;
        }
        unsigned int slot = filled < k ? filled : k - 1;
        while (slot > 0 && best[slot - 1] < score) {
            best[slot] = best[slot - 1];
            best_index[slot] = best_index[slot - 1];
            --slot;
        }
        best[slot] = score;
        best_index[slot] = has_index ? in_index[p] : (unsigned int)p;
        if (filled < k) {
            ++filled;
        }
    }
}

}
