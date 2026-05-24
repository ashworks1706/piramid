# Build Rust server
FROM rust:1.85-slim AS rust-builder

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY . .

RUN cargo build --release --bin piramid

# Runtime
FROM debian:bookworm-slim

LABEL org.opencontainers.image.title="Piramid" \
      org.opencontainers.image.description="All-in-one binary for distributed model inference, retrieval, and vector search." \
      org.opencontainers.image.source="https://github.com/ashworks1706/piramid" \
      org.opencontainers.image.url="https://piramiddb.com" \
      org.opencontainers.image.licenses="Apache-2.0"

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
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
