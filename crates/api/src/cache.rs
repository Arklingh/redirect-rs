use chrono::Utc;
use redirect_core::models::Link;
use redis::{AsyncCommands, aio::ConnectionManager};

const MAX_TTL_SECS: u64 = 300;

/// Optional Redis cache of shortcode -> link. Every failure is logged and
/// treated as a miss, so Redis being down never breaks redirects.
#[derive(Clone)]
pub struct LinkCache {
    conn: ConnectionManager,
}

fn key(shortcode: &str) -> String {
    format!("link:{shortcode}")
}

/// Cache lifetime: at most `MAX_TTL_SECS`, never beyond the link's expiry.
/// `None` means the link is already expired and should not be cached.
fn ttl_secs(link: &Link) -> Option<u64> {
    match link.expires_at {
        None => Some(MAX_TTL_SECS),
        Some(t) => {
            let left = (t - Utc::now()).num_seconds();
            (left > 0).then(|| (left as u64).min(MAX_TTL_SECS))
        }
    }
}

impl LinkCache {
    pub async fn connect(url: &str) -> Result<Self, redis::RedisError> {
        let client = redis::Client::open(url)?;
        let conn = ConnectionManager::new(client).await?;
        Ok(Self { conn })
    }

    pub async fn get(&self, shortcode: &str) -> Option<Link> {
        let mut conn = self.conn.clone();
        match conn.get::<_, Option<String>>(key(shortcode)).await {
            Ok(Some(json)) => serde_json::from_str(&json)
                .map_err(|e| tracing::warn!(error = %e, shortcode, "bad cache entry"))
                .ok(),
            Ok(None) => None,
            Err(e) => {
                tracing::warn!(error = %e, "redis get failed");
                None
            }
        }
    }

    pub async fn set(&self, link: &Link) {
        let Some(ttl) = ttl_secs(link) else { return };
        let Ok(json) = serde_json::to_string(link) else {
            return;
        };
        let mut conn = self.conn.clone();
        if let Err(e) = conn
            .set_ex::<_, _, ()>(key(&link.shortcode), json, ttl)
            .await
        {
            tracing::warn!(error = %e, "redis set failed");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;
    use uuid::Uuid;

    fn link(expires_in: Option<Duration>) -> Link {
        Link {
            id: Uuid::nil(),
            shortcode: "x".into(),
            long_url: "https://example.com".into(),
            created_at: Utc::now(),
            expires_at: expires_in.map(|d| Utc::now() + d),
            click_count: 0,
        }
    }

    #[test]
    fn ttl_is_capped_and_respects_expiry() {
        assert_eq!(ttl_secs(&link(None)), Some(MAX_TTL_SECS));
        assert_eq!(
            ttl_secs(&link(Some(Duration::hours(1)))),
            Some(MAX_TTL_SECS)
        );
        let short = ttl_secs(&link(Some(Duration::seconds(30)))).unwrap();
        assert!((28..=30).contains(&short));
        assert_eq!(ttl_secs(&link(Some(Duration::seconds(-5)))), None);
    }
}
