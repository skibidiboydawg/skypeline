# Multi-stage build for the Skypeline relay server.
# PORT defaults to 7860 (Hugging Face/local); Render overrides it to 10000.
FROM rust:1.100-slim-bookworm AS builder

WORKDIR /app
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy workspace manifests first for dependency layer caching
COPY Cargo.toml ./
COPY Cargo.lock ./
COPY shared/Cargo.toml shared/Cargo.toml
COPY server/Cargo.toml server/Cargo.toml
COPY client/Cargo.toml client/Cargo.toml

# Create dummy sources so we only compile server + shared in Docker
RUN mkdir -p shared/src server/src client/src && \
    echo "pub fn _stub() {}" > shared/src/lib.rs && \
    echo "fn main() {}" > server/src/main.rs && \
    echo "fn main() {}" > client/src/main.rs && \
    cargo build --release -p skypeline-server || true

# Copy real shared + server sources and build final binary
COPY shared shared
COPY server server
RUN touch shared/src/lib.rs server/src/main.rs && \
    cargo build --release -p skypeline-server

FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/skypeline-server /usr/local/bin/skypeline-server

# Hugging Face Spaces runs as non-root uid 1000 on port 7860
RUN useradd -m -u 1000 user
USER user
ENV PORT=7860
EXPOSE 7860

CMD ["skypeline-server"]
