FROM rust:stable-slim AS builder

WORKDIR /app

# Cache dependency builds
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

RUN cargo build -p gb-engine --bin gb-engine-service --release

# Build the health-check binary
RUN printf '#!/bin/sh\nexec curl -sf http://localhost:8081/ > /dev/null 2>&1 || exit 1\n' > /tmp/healthcheck.sh

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends curl ca-certificates \
    && rm -rf /var/lib/apt/lists/*

RUN useradd -m -s /bin/false engine

WORKDIR /app

RUN mkdir -p /app/data && chown engine:engine /app/data

COPY --from=builder /app/target/release/gb-engine-service /usr/local/bin/gb-engine-service
COPY --from=builder /tmp/healthcheck.sh /usr/local/bin/gb-engine-healthcheck
RUN chmod +x /usr/local/bin/gb-engine-healthcheck

ENV GLOWBACK_ENGINE_ADDR=0.0.0.0:8081
ENV RUST_LOG=info

EXPOSE 8081

VOLUME ["/app/data"]

USER engine

CMD ["gb-engine-service"]
