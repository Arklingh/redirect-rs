CREATE TABLE IF NOT EXISTS links (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    shortcode   TEXT NOT NULL UNIQUE,
    long_url    TEXT NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at  TIMESTAMPTZ,
    click_count BIGINT NOT NULL DEFAULT 0
);
