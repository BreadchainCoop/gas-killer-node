# Build stage
FROM rust:1.83-slim AS builder

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    git \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/app

COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN --mount=type=secret,id=GH_PAT \
    bash -lc '\
      export CARGO_NET_GIT_FETCH_WITH_CLI=true GIT_TERMINAL_PROMPT=0; \
      GH_PAT=$(cat /run/secrets/GH_PAT); \
      git config --global url."https://github.com/".insteadOf "ssh://git@github.com/"; \
      git config --global url."https://github.com/".insteadOf "git@github.com:"; \
      printf "#!/bin/sh\ncase \"$1\" in *Username*) echo x-access-token ;; *Password*) echo $GH_PAT ;; *) echo ;; esac\n" > /tmp/askpass.sh; \
      chmod +x /tmp/askpass.sh; \
      GIT_ASKPASS=/tmp/askpass.sh cargo build --release; \
      rm -f /tmp/askpass.sh; \
      rm -rf src \
    '

COPY src ./src
RUN --mount=type=secret,id=GH_PAT \
    bash -lc '\
      export CARGO_NET_GIT_FETCH_WITH_CLI=true GIT_TERMINAL_PROMPT=0; \
      GH_PAT=$(cat /run/secrets/GH_PAT); \
      printf "#!/bin/sh\ncase \"$1\" in *Username*) echo x-access-token ;; *Password*) echo $GH_PAT ;; *) echo ;; esac\n" > /tmp/askpass.sh; \
      chmod +x /tmp/askpass.sh; \
      touch src/main.rs && GIT_ASKPASS=/tmp/askpass.sh cargo build --release; \
      rm -f /tmp/askpass.sh \
    '

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/src/app/target/release/gas-killer-node /usr/local/bin/gas-killer-node
COPY orchestrator.json /etc/avs-node/orchestrator.json

ENTRYPOINT ["/usr/local/bin/gas-killer-node"]