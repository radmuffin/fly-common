use std::time::Duration;
use crate::security::ssrf::validate_parsed_url;

/// Builds a hardened `reqwest::Client` with SSRF redirect filtering, reasonable timeouts,
/// and browser-like user agent.
pub fn build_safe_http_client(timeout: Duration, user_agent: Option<&str>) -> reqwest::Client {
    let mut headers = reqwest::header::HeaderMap::new();
    let ua = user_agent.unwrap_or("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36 FlyApp/0.1.0");
    if let Ok(ua_val) = reqwest::header::HeaderValue::from_str(ua) {
        headers.insert(reqwest::header::USER_AGENT, ua_val);
    }
    headers.insert(
        reqwest::header::ACCEPT_LANGUAGE,
        reqwest::header::HeaderValue::from_static("en-US,en;q=0.9"),
    );
    headers.insert(
        reqwest::header::ACCEPT,
        reqwest::header::HeaderValue::from_static(
            "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,image/apng,*/*;q=0.8",
        ),
    );

    // Custom redirect policy that validates the target URL on every redirect step
    let redirect_policy = reqwest::redirect::Policy::custom(|attempt| {
        if attempt.previous().len() >= 10 {
            return attempt.error("Too many redirects");
        }
        let next_url = attempt.url();
        if let Err(e) = validate_parsed_url(next_url) {
            return attempt.error(format!("SSRF Protection blocked redirect: {}", e));
        }
        attempt.follow()
    });

    reqwest::Client::builder()
        .timeout(timeout)
        .redirect(redirect_policy)
        .default_headers(headers)
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
}
