use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    routing::{get, post},
};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use sqlx::{PgPool, postgres::PgPoolOptions};
use uuid::Uuid;

#[derive(Debug, Deserialize, PartialEq)]
struct ClickEvent {
    shortcode: String,
    link_id: Uuid,
    clicked_at: DateTime<Utc>,
}

async fn healthz() -> &'static str {
    "ok"
}

async fn record_click(State(pool): State<PgPool>, Json(click): Json<ClickEvent>) -> StatusCode {
    let result =
        sqlx::query("INSERT INTO clicks (link_id, shortcode, clicked_at) VALUES ($1, $2, $3)")
            .bind(click.link_id)
            .bind(&click.shortcode)
            .bind(click.clicked_at)
            .execute(&pool)
            .await;

    match result {
        Ok(_) => StatusCode::ACCEPTED,
        Err(e) => {
            tracing::error!("failed to insert click: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}

fn app(pool: PgPool) -> Router {
    Router::new()
        .route("/clicks", post(record_click))
        .route("/healthz", get(healthz))
        .with_state(pool)
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    dotenvy::dotenv().ok();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL is not set");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("failed to connect to postgres");

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS clicks (
            id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
            link_id uuid NOT NULL,
            shortcode text NOT NULL,
            clicked_at timestamptz NOT NULL
        )",
    )
    .execute(&pool)
    .await
    .expect("failed to create clicks table");

    let port = std::env::var("PORT").unwrap_or_else(|_| "3001".to_string());
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}"))
        .await
        .expect("failed to bind");
    tracing::info!("ingestion listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app(pool)).await.unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;

    const ID: &str = "67e55044-10b1-426f-9247-bb680e5fe0c8";

    #[test]
    fn parses_valid_payload() {
        let json = format!(
            r#"{{"shortcode":"abc123","link_id":"{ID}","clicked_at":"2026-01-02T03:04:05Z"}}"#
        );
        let c: ClickEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(c.shortcode, "abc123");
        assert_eq!(c.link_id.to_string(), ID);
        assert_eq!(c.clicked_at.to_rfc3339(), "2026-01-02T03:04:05+00:00");
    }

    #[test]
    fn rejects_bad_uuid() {
        let json = r#"{"shortcode":"a","link_id":"nope","clicked_at":"2026-01-02T03:04:05Z"}"#;
        assert!(serde_json::from_str::<ClickEvent>(json).is_err());
    }

    #[test]
    fn rejects_missing_field_and_bad_timestamp() {
        let missing = format!(r#"{{"shortcode":"a","link_id":"{ID}"}}"#);
        assert!(serde_json::from_str::<ClickEvent>(&missing).is_err());
        let bad = format!(r#"{{"shortcode":"a","link_id":"{ID}","clicked_at":"yesterday"}}"#);
        assert!(serde_json::from_str::<ClickEvent>(&bad).is_err());
    }
}
