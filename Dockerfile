# Build stage
FROM rust:1.83-slim AS builder

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    git \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/app

ENV CARGO_NET_GIT_FETCH_WITH_CLI=true

COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
RUN mkdir src && echo "fn main() {}" > src/main.rs

RUN --mount=type=secret,id=GIT_AUTH_TOKEN \
    echo "DEBUG: Checking for secret..." && \
    ls -la /run/secrets/ && \
    if [ -f /run/secrets/GIT_AUTH_TOKEN ]; then \
        echo "DEBUG: Secret file exists" && \
        TOKEN=$(cat /run/secrets/GIT_AUTH_TOKEN) && \
        echo "DEBUG: Token length: ${#TOKEN}" && \
        git config --global url."https://${TOKEN}@github.com/".insteadOf "ssh://git@github.com/" && \
        git config --global url."https://${TOKEN}@github.com/".insteadOf "git@github.com:"; \
    else \
        echo "ERROR: Secret file not found at /run/secrets/GIT_AUTH_TOKEN"; \
    fi && \
    cargo build --release && \
    rm -rf src

COPY src ./src
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/src/app/target/release/gas-killer-node /usr/local/bin/gas-killer-node
COPY orchestrator.json /etc/avs-node/orchestrator.json

ENTRYPOINT ["/usr/local/bin/gas-killer-node"]