FROM rust:1.88-bookworm AS builder
WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY apps/api/Cargo.toml apps/api/Cargo.toml
COPY apps/api/src apps/api/src
COPY db/migrations db/migrations

RUN cargo build -p api --release

FROM debian:bookworm-slim
WORKDIR /app

RUN apt-get update \
  && apt-get install -y --no-install-recommends ca-certificates \
  && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/api /usr/local/bin/api
COPY --from=builder /app/target/release/migrate /usr/local/bin/migrate

ENV APP_HOST=0.0.0.0
ENV APP_PORT=3000

CMD ["api"]
