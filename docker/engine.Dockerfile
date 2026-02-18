FROM rust:1.75-slim AS builder

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

RUN cargo build -p gb-engine --bin gb-engine-service --release

FROM debian:bookworm-slim

RUN useradd -m engine

WORKDIR /app

COPY --from=builder /app/target/release/gb-engine-service /usr/local/bin/gb-engine-service

ENV GLOWBACK_ENGINE_ADDR=0.0.0.0:8081

EXPOSE 8081

USER engine

CMD ["gb-engine-service"]
