use crate::models::Link;
use sqlx::PgPool;
use uuid::Uuid;

pub async fn insert_link(
    pool: &PgPool,
    shortcode: &str,
    long_url: &str,
) -> Result<Link, sqlx::Error> {
    sqlx::query_as::<_, Link>("INSERT INTO links (shortcode, long_url) VALUES ($1, $2) RETURNING *")
        .bind(shortcode)
        .bind(long_url)
        .fetch_one(pool)
        .await
}

pub async fn get_link_by_shortcode(
    pool: &PgPool,
    shortcode: &str,
) -> Result<Option<Link>, sqlx::Error> {
    sqlx::query_as::<_, Link>("SELECT * FROM links WHERE shortcode = $1")
        .bind(shortcode)
        .fetch_optional(pool)
        .await
}

pub async fn increment_click_count(pool: &PgPool, id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE links SET click_count = click_count + 1 WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn get_link_by_id(pool: &PgPool, id: Uuid) -> Result<Option<Link>, sqlx::Error> {
    sqlx::query_as::<_, Link>("SELECT * FROM links WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
}
