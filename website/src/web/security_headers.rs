use axum::{
    Extension, body::Body, extract::Request, http::header, middleware::Next, response::Response,
};

use crate::models::CspNonce;

/// Middleware to add security headers to all responses
pub async fn add_security_headers(
    csp_nonce: Extension<CspNonce>,
    req: Request,
    next: Next,
) -> Response<Body> {
    let mut response = next.run(req).await;

    let csp_value = format!(
        "default-src 'self'; \
             script-src 'self' 'nonce-{}' 'unsafe-eval' https://www.google.com/recaptcha/ https://www.gstatic.com/recaptcha/; \
             style-src 'self' 'unsafe-inline'; \
             img-src 'self' data:; \
             font-src 'self'; \
             connect-src 'self' https://www.google.com/recaptcha/; \
             frame-src https://www.google.com/recaptcha/ https://recaptcha.google.com/recaptcha/; \
             frame-ancestors 'none'; \
             base-uri 'self'; \
             form-action 'self'",
        csp_nonce.nonce
    );

    let headers = response.headers_mut();

    // Prevent clickjacking attacks
    headers.insert(
        header::HeaderName::from_static("x-frame-options"),
        header::HeaderValue::from_static("DENY"),
    );

    // Prevent MIME type sniffing
    headers.insert(
        header::HeaderName::from_static("x-content-type-options"),
        header::HeaderValue::from_static("nosniff"),
    );

    // Enable XSS protection (legacy, but doesn't hurt)
    headers.insert(
        header::HeaderName::from_static("x-xss-protection"),
        header::HeaderValue::from_static("1; mode=block"),
    );

    // Control referrer information
    headers.insert(
        header::REFERRER_POLICY,
        header::HeaderValue::from_static("strict-origin-when-cross-origin"),
    );

    // Content Security Policy
    // Allows resources only from same origin, with inline styles for templates
    // and data URIs for images
    // TODO: Find a way to allow alpine-js to work without 'unsafe-eval' in script-src
    if let Ok(value) = header::HeaderValue::from_str(&csp_value) {
        headers.insert(header::CONTENT_SECURITY_POLICY, value);
    }

    response
}
