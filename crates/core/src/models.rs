use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Link {
    pub id: Uuid,
    pub shortcode: String,
    pub long_url: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub click_count: i64,
}

#[derive(Debug, Deserialize)]
pub struct CreateLinkRequest {
    pub long_url: String,
    pub custom_alias: Option<String>,
}
