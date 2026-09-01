use axum::{extract::Request, middleware::Next, response::Response};
use tower_http::cors::{Any, CorsLayer};

/// Middleware that sets standard security response headers:
/// - X-Content-Type-Options: nosniff
/// - X-Frame-Options: SAMEORIGIN
/// - Referrer-Policy: strict-origin-when-cross-origin
pub async fn set_security_headers(req: Request, next: Next) -> Response {
    let mut response = next.run(req).await;
    let headers = response.headers_mut();
    headers.insert(
        axum::http::header::X_CONTENT_TYPE_OPTIONS,
        axum::http::HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        axum::http::header::X_FRAME_OPTIONS,
        axum::http::HeaderValue::from_static("SAMEORIGIN"),
    );
    headers.insert(
        axum::http::header::REFERRER_POLICY,
        axum::http::HeaderValue::from_static("strict-origin-when-cross-origin"),
    );
    response
}

/// Standard permissive CORS configuration suitable for PWA/SPAs using custom headers.
pub fn standard_cors_layer() -> CorsLayer {
    CorsLayer::new()
        .allow_methods([
            axum::http::Method::GET,
            axum::http::Method::POST,
            axum::http::Method::PUT,
            axum::http::Method::PATCH,
            axum::http::Method::DELETE,
            axum::http::Method::OPTIONS,
        ])
        .allow_headers([
            axum::http::header::CONTENT_TYPE,
            axum::http::header::ACCEPT,
            axum::http::header::AUTHORIZATION,
            axum::http::HeaderName::from_static("x-user-token"),
        ])
        .allow_origin(Any)
}
