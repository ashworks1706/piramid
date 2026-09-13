# CPU image. Built and pushed by .github/workflows/cd.yml.
#
# cargo-chef caches dependency builds in their own layer. The builder uses stable Rust.

FROM rust:1-slim AS chef
RUN cargo install cargo-chef --locked
WORKDIR /app

# Record the dependency graph only.
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
# Cached unless a manifest changes.
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo build --release --locked --bin piramid

FROM debian:bookworm-slim AS runtime

LABEL org.opencontainers.image.title="Piramid" \
      org.opencontainers.image.description="Inference engine for retrieval systems." \
      org.opencontainers.image.source="https://github.com/ashworks1706/piramid" \
      org.opencontainers.image.url="https://piramiddb.com" \
      org.opencontainers.image.licenses="Apache-2.0"

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*

# Unprivileged; the data volume is chowned to this user.
RUN useradd --system --create-home --uid 10001 piramid
WORKDIR /app
COPY --from=builder /app/target/release/piramid /usr/local/bin/piramid
RUN mkdir -p /data && chown piramid:piramid /data
USER piramid

ENV PIRAMID__STARTUP__BIND=0.0.0.0:6333 \
    PIRAMID__STARTUP__DATA_DIR=/data \
    RUST_LOG=info

VOLUME ["/data"]
EXPOSE 6333

HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -fsS http://localhost:6333/api/health || exit 1

ENTRYPOINT ["piramid"]
CMD ["serve"]
