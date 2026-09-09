use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::Redirect,
};
use nanoid::nanoid;
use redirect_core::{
    db,
    models::{CreateLinkRequest, Link},
};
use sqlx::PgPool;
use uuid::Uuid;

pub async fn create_link(
    State(pool): State<PgPool>,
    Json(payload): Json<CreateLinkRequest>,
) -> Result<Json<Link>, StatusCode> {
    let shortcode = payload.custom_alias.unwrap_or_else(|| nanoid!(7));

    let link = db::insert_link(&pool, &shortcode, &payload.long_url)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(link))
}

pub async fn get_link(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
) -> Result<Json<Link>, StatusCode> {
    db::get_link_by_id(&pool, id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

pub async fn redirect(
    State(pool): State<PgPool>,
    Path(shortcode): Path<String>,
) -> Result<Redirect, StatusCode> {
    let link = db::get_link_by_shortcode(&pool, &shortcode)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // TODO: check Redis cache before hitting Postgres
    // TODO: fire gPRC call to ingestion service to record the click

    Ok(Redirect::temporary(&link.long_url))
}
