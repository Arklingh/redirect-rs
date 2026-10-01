use axum::{
    Router,
    routing::{get, post},
};
use sqlx::{PgPool, postgres::PgPoolOptions};

mod cache;
mod error;
mod handlers;
mod validation;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub http: reqwest::Client,
    pub ingestion_url: Option<String>,
    pub cache: Option<cache::LinkCache>,
}

pub fn app(state: AppState) -> Router {
    Router::new()
        .route("/healthz", get(handlers::healthz))
        .route("/links", post(handlers::create_link))
        .route("/links/{id}", get(handlers::get_link))
        .route("/{shortcode}", get(handlers::redirect))
        .with_state(state)
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL is not set");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("failed to connect to postgres");

    sqlx::migrate!("../../migrations")
        .run(&pool)
        .await
        .expect("failed to run migrations");

    let ingestion_url = std::env::var("INGESTION_URL")
        .ok()
        .filter(|s| !s.is_empty());
    let cache = match std::env::var("REDIS_URL").ok().filter(|s| !s.is_empty()) {
        Some(url) => match cache::LinkCache::connect(&url).await {
            Ok(c) => {
                tracing::info!("redis cache enabled");
                Some(c)
            }
            Err(e) => {
                tracing::warn!(error = %e, "redis unavailable, running without cache");
                None
            }
        },
        None => None,
    };
    let state = AppState {
        pool,
        http: reqwest::Client::new(),
        ingestion_url,
        cache,
    };

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3000);
    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port))
        .await
        .unwrap();
    tracing::info!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app(state)).await.unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;

    fn state(pool: PgPool) -> AppState {
        AppState {
            pool,
            http: reqwest::Client::new(),
            ingestion_url: None,
            cache: None,
        }
    }

    fn post_json(body: &str) -> Request<Body> {
        Request::post("/links")
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap()
    }

    fn get_req(path: &str) -> Request<Body> {
        Request::get(path).body(Body::empty()).unwrap()
    }

    async fn status(app: &Router, req: Request<Body>) -> StatusCode {
        app.clone().oneshot(req).await.unwrap().status()
    }

    // DB-backed tests: run with `DATABASE_URL=... cargo test -p api -- --ignored`.
    #[sqlx::test(migrations = "../../migrations")]
    #[ignore = "requires DATABASE_URL"]
    async fn create_redirect_conflict_validation(pool: PgPool) {
        let app = app(state(pool.clone()));
        assert_eq!(status(&app, get_req("/healthz")).await, StatusCode::OK);

        let ok = r#"{"long_url":"https://example.com","custom_alias":"abc"}"#;
        assert_eq!(status(&app, post_json(ok)).await, StatusCode::OK);
        assert_eq!(status(&app, post_json(ok)).await, StatusCode::CONFLICT);
        assert_eq!(
            status(&app, post_json(r#"{"long_url":"ftp://x.com"}"#)).await,
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            status(
                &app,
                post_json(r#"{"long_url":"https://x.com","custom_alias":"a b"}"#)
            )
            .await,
            StatusCode::BAD_REQUEST
        );

        assert_eq!(
            status(&app, get_req("/abc")).await,
            StatusCode::TEMPORARY_REDIRECT
        );
        assert_eq!(status(&app, get_req("/nope")).await, StatusCode::NOT_FOUND);

        // click count is incremented by a spawned task
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        let n: i64 = sqlx::query_scalar("SELECT click_count FROM links WHERE shortcode='abc'")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(n, 1);
    }

    #[sqlx::test(migrations = "../../migrations")]
    #[ignore = "requires DATABASE_URL"]
    async fn expired_link_is_gone(pool: PgPool) {
        sqlx::query("INSERT INTO links (shortcode, long_url, expires_at) VALUES ('old','https://example.com', now() - interval '1 hour')")
            .execute(&pool)
            .await
            .unwrap();
        let app = app(state(pool));
        assert_eq!(status(&app, get_req("/old")).await, StatusCode::GONE);
    }
}
