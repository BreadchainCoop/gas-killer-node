# Build stage
FROM rust:1.83-slim AS builder

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    git \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/app

COPY Cargo.toml Cargo.lock rust-toolchain.toml ./

RUN --mount=type=secret,id=GIT_AUTH_TOKEN \
    if [ -f /run/secrets/GIT_AUTH_TOKEN ]; then \
        TOKEN=$(cat /run/secrets/GIT_AUTH_TOKEN) && \
        git config --global url."https://${TOKEN}@github.com/".insteadOf "https://github.com/"; \
    else \
        echo "ERROR: Secret file not found at /run/secrets/GIT_AUTH_TOKEN" && exit 1; \
    fi

# Prefetch dependencies to warm cargo cache without producing a stub binary
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/src/app/target \
    cargo fetch

COPY src ./src
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/src/app/target \
    cargo build --release

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/src/app/target/release/gas-killer-node /usr/local/bin/gas-killer-node
COPY orchestrator.json /etc/avs-node/orchestrator.json

ENTRYPOINT ["/usr/local/bin/gas-killer-node"]