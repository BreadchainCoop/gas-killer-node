# Build stage
FROM rust:1.83-slim AS builder

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    git \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/app

RUN --mount=type=secret,id=GIT_AUTH_TOKEN \
    git config --global url."https://$(cat /run/secrets/GIT_AUTH_TOKEN)@github.com/".insteadOf "https://github.com/"

ENV CARGO_NET_GIT_FETCH_WITH_CLI=true

COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
RUN mkdir src && echo "fn main() {}" > src/main.rs

RUN --mount=type=secret,id=GIT_AUTH_TOKEN \
    git config --global url."https://$(cat /run/secrets/GIT_AUTH_TOKEN)@github.com/".insteadOf "https://github.com/" && \
    cargo build --release && \
    rm -rf src

COPY src ./src
RUN --mount=type=secret,id=GIT_AUTH_TOKEN \
    git config --global url."https://$(cat /run/secrets/GIT_AUTH_TOKEN)@github.com/".insteadOf "https://github.com/" && \
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