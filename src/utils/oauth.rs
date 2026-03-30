use url::Url;

/// Validates that the provided redirect_uri matches or is compatible with the registered redirect_uri
/// Rules:
/// - Exact match is always allowed
/// - Prefix match: provided URI must start with the registered URI
pub fn validate_redirect_uri(registered: &str, provided: &str) -> bool {
    // Exact match
    if registered == provided {
        return true;
    }

    // Parse both URLs to validate they're proper URLs
    let Ok(registered_url) = Url::parse(registered) else {
        return false;
    };
    let Ok(provided_url) = Url::parse(provided) else {
        return false;
    };

    // Scheme and host must match exactly
    if registered_url.scheme() != provided_url.scheme() {
        return false;
    }
    if registered_url.host_str() != provided_url.host_str() {
        return false;
    }
    if registered_url.port() != provided_url.port() {
        return false;
    }

    // Prefix match on path
    provided_url.path().starts_with(registered_url.path())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exact_match() {
        assert!(validate_redirect_uri(
            "https://example.com/callback",
            "https://example.com/callback"
        ));
    }

    #[test]
    fn test_prefix_match() {
        assert!(validate_redirect_uri(
            "https://example.com/callback",
            "https://example.com/callback/page1"
        ));
        assert!(validate_redirect_uri(
            "https://example.com/callback",
            "https://example.com/callback/nested/path"
        ));
    }

    #[test]
    fn test_scheme_mismatch() {
        assert!(!validate_redirect_uri(
            "https://example.com/callback",
            "http://example.com/callback"
        ));
    }

    #[test]
    fn test_host_mismatch() {
        assert!(!validate_redirect_uri(
            "https://example.com/callback",
            "https://evil.com/callback"
        ));
    }

    #[test]
    fn test_path_mismatch() {
        assert!(!validate_redirect_uri(
            "https://example.com/callback",
            "https://example.com/other"
        ));
    }

    #[test]
    fn test_path_prefix_required() {
        // "/callback" should not match "/call"
        assert!(!validate_redirect_uri(
            "https://example.com/callback",
            "https://example.com/call"
        ));
    }
}
