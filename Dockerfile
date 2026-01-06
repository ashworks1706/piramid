# =============================================================================
# PIRAMID DOCKERFILE
# Full Rust server + Next.js dashboard
# =============================================================================

# -----------------------------------------------------------------------------
# Stage 1: Build the Next.js dashboard
# -----------------------------------------------------------------------------
FROM node:20-slim AS dashboard-builder

WORKDIR /app/dashboard

COPY dashboard/package.json dashboard/package-lock.json ./
RUN npm ci

COPY dashboard ./
RUN npm run build

# -----------------------------------------------------------------------------
# Stage 2: Build Rust server
# -----------------------------------------------------------------------------
FROM rust:1.75-slim AS rust-builder

WORKDIR /app

RUN apt-get update && apt-get install -y pkg-config && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN cargo build --release --bin piramid-server

# -----------------------------------------------------------------------------
# Stage 3: Runtime
# -----------------------------------------------------------------------------
FROM debian:bookworm-slim

WORKDIR /app

RUN apt-get update && apt-get install -y ca-certificates curl && rm -rf /var/lib/apt/lists/*

COPY --from=rust-builder /app/target/release/piramid-server ./piramid-server
COPY --from=dashboard-builder /app/dashboard/out ./dashboard

RUN mkdir -p /app/data

ENV PORT=6333
ENV DATA_DIR=/app/data
ENV RUST_LOG=info

EXPOSE 6333

HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:6333/api/health || exit 1

CMD ["./piramid-server"]

