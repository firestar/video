# syntax=docker/dockerfile:1

FROM rust:1.79 AS builder
WORKDIR /app

# Build context = project root (docker-compose sets it to `..`).
# Copy workspace manifest + lockfile, then the crate sources.
COPY Cargo.toml Cargo.lock ./
COPY backend ./backend

# Build api + worker. Runtime queries (no sqlx compile-time macros),
# so no DATABASE_URL is needed at build time.
RUN cargo build --release --workspace --bin api --bin worker

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ffmpeg curl && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/api /usr/local/bin/api
COPY --from=builder /app/target/release/worker /usr/local/bin/worker
EXPOSE 8080
HEALTHCHECK CMD curl -f http://localhost:8080/health
CMD ["api"]
