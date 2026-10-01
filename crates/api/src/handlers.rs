use axum::{
    Json,
    extract::{Path, State},
    response::Redirect,
};
use chrono::Utc;
use nanoid::nanoid;
use redirect_core::{
    db,
    models::{CreateLinkRequest, Link},
};
use serde_json::json;
use uuid::Uuid;

use crate::{AppState, error::ApiError, validation};

pub async fn healthz() -> &'static str {
    "ok"
}

pub async fn create_link(
    State(state): State<AppState>,
    Json(payload): Json<CreateLinkRequest>,
) -> Result<Json<Link>, ApiError> {
    validation::validate_long_url(&payload.long_url).map_err(ApiError::BadRequest)?;
    let shortcode = match payload.custom_alias {
        Some(alias) => {
            validation::validate_alias(&alias).map_err(ApiError::BadRequest)?;
            alias
        }
        None => nanoid!(7),
    };

    match db::insert_link(&state.pool, &shortcode, &payload.long_url).await {
        Ok(link) => Ok(Json(link)),
        Err(sqlx::Error::Database(e)) if e.is_unique_violation() => Err(ApiError::Conflict(
            format!("shortcode '{shortcode}' already exists"),
        )),
        Err(e) => Err(e.into()),
    }
}

pub async fn get_link(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Link>, ApiError> {
    db::get_link_by_id(&state.pool, id)
        .await?
        .map(Json)
        .ok_or(ApiError::NotFound)
}

pub async fn redirect(
    State(state): State<AppState>,
    Path(shortcode): Path<String>,
) -> Result<Redirect, ApiError> {
    // NOTE: Redis caching of shortcode lookups is deferred.
    let link = db::get_link_by_shortcode(&state.pool, &shortcode)
        .await?
        .ok_or(ApiError::NotFound)?;

    if link.expires_at.is_some_and(|t| t <= Utc::now()) {
        return Err(ApiError::Gone);
    }

    record_click(&state, &link);
    Ok(Redirect::temporary(&link.long_url))
}

/// Fire-and-forget click recording: bump the counter and notify ingestion.
fn record_click(state: &AppState, link: &Link) {
    let state = state.clone();
    let link_id = link.id;
    let shortcode = link.shortcode.clone();
    let clicked_at = Utc::now();
    tokio::spawn(async move {
        if let Err(e) = db::increment_click_count(&state.pool, link_id).await {
            tracing::error!(error = %e, %link_id, "failed to increment click_count");
        }
        if let Some(base) = &state.ingestion_url {
            let url = format!("{}/clicks", base.trim_end_matches('/'));
            let body = json!({
                "shortcode": shortcode,
                "link_id": link_id,
                "clicked_at": clicked_at,
            });
            if let Err(e) = state.http.post(&url).json(&body).send().await {
                tracing::warn!(error = %e, %url, "failed to post click to ingestion");
            }
        }
    });
}
