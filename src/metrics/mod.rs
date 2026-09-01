use axum::{
    extract::State,
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

/// Lightweight Prometheus-compatible in-process metrics for Fly.io applications.
/// No external prometheus crate needed — pure atomic counters.
#[derive(Clone)]
pub struct FlyMetrics {
    requests_2xx: Arc<AtomicU64>,
    requests_4xx: Arc<AtomicU64>,
    requests_5xx: Arc<AtomicU64>,
    uptime_start: Arc<Instant>,
    active_connections: Arc<AtomicI64>,
}

impl Default for FlyMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl FlyMetrics {
    pub fn new() -> Self {
        Self {
            requests_2xx: Arc::new(AtomicU64::new(0)),
            requests_4xx: Arc::new(AtomicU64::new(0)),
            requests_5xx: Arc::new(AtomicU64::new(0)),
            uptime_start: Arc::new(Instant::now()),
            active_connections: Arc::new(AtomicI64::new(0)),
        }
    }

    pub fn inc_request(&self, status: u16) {
        if (200..300).contains(&status) {
            self.requests_2xx.fetch_add(1, Ordering::Relaxed);
        } else if (400..500).contains(&status) {
            self.requests_4xx.fetch_add(1, Ordering::Relaxed);
        } else if (500..600).contains(&status) {
            self.requests_5xx.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub fn inc_active(&self) {
        self.active_connections.fetch_add(1, Ordering::Relaxed);
    }

    pub fn dec_active(&self) {
        self.active_connections.fetch_sub(1, Ordering::Relaxed);
    }

    pub fn render_prometheus(&self) -> String {
        let uptime = self.uptime_start.elapsed().as_secs();
        let conn = self.active_connections.load(Ordering::Relaxed);
        let r2xx = self.requests_2xx.load(Ordering::Relaxed);
        let r4xx = self.requests_4xx.load(Ordering::Relaxed);
        let r5xx = self.requests_5xx.load(Ordering::Relaxed);

        format!(
            "# HELP fly_requests_total Total HTTP requests processed\n\
             # TYPE fly_requests_total counter\n\
             fly_requests_total{{status=\"2xx\"}} {r2xx}\n\
             fly_requests_total{{status=\"4xx\"}} {r4xx}\n\
             fly_requests_total{{status=\"5xx\"}} {r5xx}\n\
             # HELP fly_active_connections Current active HTTP connections\n\
             # TYPE fly_active_connections gauge\n\
             fly_active_connections {conn}\n\
             # HELP fly_uptime_seconds Application uptime in seconds\n\
             # TYPE fly_uptime_seconds gauge\n\
             fly_uptime_seconds {uptime}\n"
        )
    }
}

pub async fn metrics_middleware(
    State(metrics): State<FlyMetrics>,
    req: axum::extract::Request,
    next: Next,
) -> Response {
    metrics.inc_active();
    let res = next.run(req).await;
    metrics.dec_active();
    metrics.inc_request(res.status().as_u16());
    res
}

/// GET handler for /metrics endpoint (Prometheus text format)
pub async fn metrics_handler(State(metrics): State<FlyMetrics>) -> impl IntoResponse {
    let body = metrics.render_prometheus();
    (
        [(
            axum::http::header::CONTENT_TYPE,
            "text/plain; version=0.0.4; charset=utf-8",
        )],
        body,
    )
}
