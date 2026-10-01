# redirect-rs

A URL shortener written in Rust. It creates short codes for long URLs, redirects visitors to the original URL, and records click events for analytics.

## Architecture

Cargo workspace:

- `crates/core` - shared domain logic (`redirect_core`).
- `crates/api` - public HTTP service (axum, port 3000). Creates and looks up links in Postgres and serves redirects. Reports clicks to the ingestion service at `INGESTION_URL`.
- `crates/ingestion` - click ingestion service (axum, port 3001, `PORT` to override). Receives click events and stores them in the `clicks` table (created at startup if missing, no foreign key to `links`).
- `crates/wasm-utils` - reserved, currently an unspecified and unused stub. Nothing depends on it.

Storage is PostgreSQL.

## Endpoints

### api (port 3000)

| Method | Path | Description |
| --- | --- | --- |
| POST | `/links` | Create a link. Body: `{"long_url": "https://...", "custom_alias": "optional"}` |
| GET | `/links/{id}` | Fetch a link by id |
| GET | `/{shortcode}` | Redirect to the long URL |
| GET | `/healthz` | Health check |

### ingestion (port 3001)

| Method | Path | Description |
| --- | --- | --- |
| POST | `/clicks` | Record a click. Body: `{"shortcode": "abc", "link_id": "<uuid>", "clicked_at": "<RFC 3339 timestamp>"}` |
| GET | `/healthz` | Health check |

## Configuration

See `.env.example`. Variables: `DATABASE_URL` (both services), `INGESTION_URL` (api), `PORT` (ingestion, default 3001).

## Running locally

```sh
cp .env.example .env
docker compose up -d postgres
cargo run -p ingestion   # terminal 1
cargo run -p api         # terminal 2
```

Tests: `cargo test -p ingestion`.

## Running with Docker Compose

```sh
docker compose up --build
```

This starts Postgres (with a healthcheck), the api on http://localhost:3000 and ingestion on http://localhost:3001. Images are built from the multi-stage `Dockerfile` (targets `api` and `ingestion`).
