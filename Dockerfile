# Build Rust server
FROM rust:1.85-slim AS rust-builder

WORKDIR /app

# Install build dependencies (OpenSSL required by reqwest)
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY benches ./benches

RUN cargo build --release --bin piramid

# Runtime
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=rust-builder /app/target/release/piramid ./piramid

RUN mkdir -p /app/data

ENV PORT=6333
ENV DATA_DIR=/app/data
ENV RUST_LOG=info

EXPOSE 6333

HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:6333/api/health || exit 1

CMD ["./piramid", "serve"]
