use axum::{
    Router,
    routing::{get, post},
};
use sqlx::postgres::PgPoolOptions;

mod handlers;

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

    let app = Router::new()
        .route("/links", post(handlers::create_link))
        .route("/links/:id", get(handlers::get_link))
        .route("/:shortcode", get(handlers::redirect))
        .with_state(pool);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    tracing::info!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}
