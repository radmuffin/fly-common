use axum::{
    extract::State,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Clone, Debug, PartialEq)]
pub enum RateLimitResult {
    Allowed,
    Blocked { retry_after_secs: u64 },
}

struct RateBucketState {
    last_reset: Instant,
    tokens_remaining: u32,
}

/// In-memory sliding window token bucket rate limiter.
/// Uses std HashMap + Mutex for per-IP state.
/// No Redis or external services needed.
#[derive(Clone)]
pub struct RateLimiter {
    state: Arc<Mutex<HashMap<String, RateBucketState>>>,
    max_tokens: u32,
    refill_interval: Duration,
    penalty_box: Arc<Mutex<HashMap<String, Instant>>>,
    penalty_duration: Duration,
}

impl RateLimiter {
    pub fn new(max_requests_per_window: u32, window: Duration) -> Self {
        Self {
            state: Arc::new(Mutex::new(HashMap::new())),
            max_tokens: max_requests_per_window,
            refill_interval: window,
            penalty_box: Arc::new(Mutex::new(HashMap::new())),
            penalty_duration: Duration::from_secs(60),
        }
    }

    pub fn check_and_consume(&self, ip: &str) -> RateLimitResult {
        if self.is_in_penalty_box(ip) {
            let pb = self.penalty_box.lock().unwrap();
            if let Some(expiry) = pb.get(ip) {
                let now = Instant::now();
                if *expiry > now {
                    return RateLimitResult::Blocked {
                        retry_after_secs: expiry.duration_since(now).as_secs(),
                    };
                }
            }
        }

        let mut state = self.state.lock().unwrap();
        let now = Instant::now();

        let bucket = state
            .entry(ip.to_string())
            .or_insert_with(|| RateBucketState {
                last_reset: now,
                tokens_remaining: self.max_tokens,
            });

        if now.duration_since(bucket.last_reset) >= self.refill_interval {
            bucket.tokens_remaining = self.max_tokens;
            bucket.last_reset = now;
        }

        if bucket.tokens_remaining > 0 {
            bucket.tokens_remaining -= 1;
            RateLimitResult::Allowed
        } else {
            let retry_after = self
                .refill_interval
                .saturating_sub(now.duration_since(bucket.last_reset));
            RateLimitResult::Blocked {
                retry_after_secs: retry_after.as_secs().max(1),
            }
        }
    }

    pub fn penalty_box_ip(&self, ip: &str) {
        let mut pb = self.penalty_box.lock().unwrap();
        pb.insert(ip.to_string(), Instant::now() + self.penalty_duration);
    }

    pub fn is_in_penalty_box(&self, ip: &str) -> bool {
        let mut pb = self.penalty_box.lock().unwrap();
        if let Some(expiry) = pb.get(ip) {
            if *expiry > Instant::now() {
                return true;
            } else {
                pb.remove(ip);
            }
        }
        false
    }
}

pub async fn rate_limit_middleware(
    State(limiter): State<RateLimiter>,
    req: axum::extract::Request,
    next: Next,
) -> Response {
    let mut client_ip = "127.0.0.1".to_string();
    if let Some(forwarded_for) = req.headers().get("x-forwarded-for") {
        if let Ok(ip_str) = forwarded_for.to_str() {
            client_ip = ip_str.split(',').next().unwrap_or("").trim().to_string();
        }
    } else if let Some(real_ip) = req.headers().get("x-real-ip") {
        if let Ok(ip_str) = real_ip.to_str() {
            client_ip = ip_str.trim().to_string();
        }
    }

    match limiter.check_and_consume(&client_ip) {
        RateLimitResult::Allowed => next.run(req).await,
        RateLimitResult::Blocked { retry_after_secs } => {
            let mut res = StatusCode::TOO_MANY_REQUESTS.into_response();
            res.headers_mut().insert(
                axum::http::header::RETRY_AFTER,
                axum::http::HeaderValue::from_str(&retry_after_secs.to_string()).unwrap(),
            );
            res
        }
    }
}
