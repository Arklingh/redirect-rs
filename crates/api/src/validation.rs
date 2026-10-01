use url::Url;

pub const MAX_ALIAS_LEN: usize = 32;

pub fn validate_long_url(s: &str) -> Result<(), String> {
    let url = Url::parse(s).map_err(|e| format!("invalid long_url: {e}"))?;
    match url.scheme() {
        "http" | "https" if url.host_str().is_some() => Ok(()),
        "http" | "https" => Err("long_url must have a host".into()),
        _ => Err("long_url must use http or https".into()),
    }
}

pub fn validate_alias(s: &str) -> Result<(), String> {
    if s.is_empty() {
        return Err("custom_alias must not be empty".into());
    }
    if s.len() > MAX_ALIAS_LEN {
        return Err(format!(
            "custom_alias must be at most {MAX_ALIAS_LEN} characters"
        ));
    }
    if !s
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err("custom_alias may only contain letters, digits, '-' and '_'".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urls() {
        assert!(validate_long_url("https://example.com/a?b=1").is_ok());
        assert!(validate_long_url("http://example.com").is_ok());
        assert!(validate_long_url("ftp://example.com").is_err());
        assert!(validate_long_url("javascript:alert(1)").is_err());
        assert!(validate_long_url("not a url").is_err());
        assert!(validate_long_url("").is_err());
    }

    #[test]
    fn aliases() {
        assert!(validate_alias("my-alias_1").is_ok());
        assert!(validate_alias(&"a".repeat(32)).is_ok());
        assert!(validate_alias(&"a".repeat(33)).is_err());
        assert!(validate_alias("").is_err());
        assert!(validate_alias("has space").is_err());
        assert!(validate_alias("slash/es").is_err());
        assert!(validate_alias("\u{fc}n\u{ef}").is_err());
    }
}
