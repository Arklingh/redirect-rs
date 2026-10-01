FROM rust:1-bookworm AS builder
WORKDIR /app
COPY . .
RUN cargo build --release -p api -p ingestion

FROM debian:bookworm-slim AS runtime-base
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

FROM runtime-base AS api
COPY --from=builder /app/target/release/api /usr/local/bin/api
EXPOSE 3000
CMD ["api"]

FROM runtime-base AS ingestion
COPY --from=builder /app/target/release/ingestion /usr/local/bin/ingestion
EXPOSE 3001
CMD ["ingestion"]
