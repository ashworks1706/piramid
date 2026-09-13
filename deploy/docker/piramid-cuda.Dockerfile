# CUDA image. Run with --gpus all and the NVIDIA container toolkit.

FROM nvidia/cuda:13.3.1-devel-ubuntu22.04 AS builder

ARG RUST_VERSION=stable
RUN apt-get update \
    && apt-get install -y --no-install-recommends build-essential curl ca-certificates pkg-config \
    && rm -rf /var/lib/apt/lists/*
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
    | sh -s -- -y --default-toolchain ${RUST_VERSION} --profile minimal
ENV PATH="/root/.cargo/bin:${PATH}"

WORKDIR /app
COPY . .
RUN cargo build --release --locked --bin piramid --features gpu-cuda

FROM nvidia/cuda:13.3.1-runtime-ubuntu22.04 AS runtime

LABEL org.opencontainers.image.title="Piramid (CUDA)" \
      org.opencontainers.image.description="Inference engine for retrieval systems, CUDA build." \
      org.opencontainers.image.source="https://github.com/ashworks1706/piramid" \
      org.opencontainers.image.licenses="Apache-2.0"

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*

RUN useradd --system --create-home --uid 10001 piramid
WORKDIR /app
COPY --from=builder /app/target/release/piramid /usr/local/bin/piramid
RUN mkdir -p /data && chown piramid:piramid /data
USER piramid

ENV PIRAMID__STARTUP__BIND=0.0.0.0:6333 \
    PIRAMID__STARTUP__DATA_DIR=/data \
    RUST_LOG=info \
    PIRAMID__STARTUP__HARDWARE__PROFILE=gpu \
    PIRAMID__RUNTIME__EXECUTION=gpu

VOLUME ["/data"]
EXPOSE 6333

HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -fsS http://localhost:6333/api/health || exit 1

ENTRYPOINT ["piramid"]
CMD ["serve"]
