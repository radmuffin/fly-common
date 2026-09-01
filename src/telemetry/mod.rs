//! W3C Trace Context (traceparent) header parser and propagator.
//!
//! Spec: <https://www.w3.org/TR/trace-context/>
//!
//! traceparent format: `{version}-{trace-id}-{parent-id}-{flags}`
//! Example: `00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01`

use axum::{
    extract::FromRequestParts,
    http::{request::Parts, HeaderValue, Request, StatusCode},
    middleware::Next,
    response::Response,
};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// Generate a pseudo-random 64-bit value using timestamp nanos XOR'd with an
/// incrementing counter. No external crate required.
fn pseudo_random_u64() -> u64 {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos() as u64;
    let count = COUNTER.fetch_add(1, Ordering::Relaxed);
    // Mix with a multiplicative hash to spread bits
    nanos
        .wrapping_mul(6364136223846793005)
        .wrapping_add(count.wrapping_mul(1442695040888963407))
        ^ nanos.rotate_left(17)
        ^ count.rotate_right(31)
}

/// Parsed and validated W3C Trace Context carried by a request or response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceContext {
    /// 32 lowercase hex characters — the W3C trace-id field.
    pub trace_id: String,
    /// 16 lowercase hex characters — the W3C parent-id / span-id field.
    pub span_id: String,
    /// Trace flags byte. Bit 0 (`0x01`) means "sampled".
    pub flags: u8,
}

impl TraceContext {
    /// Parse a W3C `traceparent` header value.
    ///
    /// Returns `None` if the header does not conform to the
    /// `version-trace-id-parent-id-flags` format or if any segment has the
    /// wrong length / contains non-hex characters.
    pub fn from_header(header: &str) -> Option<Self> {
        let parts: Vec<&str> = header.splitn(4, '-').collect();
        if parts.len() < 4 {
            return None;
        }
        let (_version, trace_id, span_id, flags_hex) = (parts[0], parts[1], parts[2], parts[3]);

        // Validate lengths and hex content
        if trace_id.len() != 32 || !trace_id.chars().all(|c| c.is_ascii_hexdigit()) {
            return None;
        }
        if span_id.len() != 16 || !span_id.chars().all(|c| c.is_ascii_hexdigit()) {
            return None;
        }
        let flags = u8::from_str_radix(flags_hex, 16).ok()?;

        Some(TraceContext {
            trace_id: trace_id.to_lowercase(),
            span_id: span_id.to_lowercase(),
            flags,
        })
    }

    /// Serialise this context as a W3C `traceparent` header value.
    pub fn to_header(&self) -> String {
        format!("00-{}-{}-{:02x}", self.trace_id, self.span_id, self.flags)
    }

    /// Generate a brand-new root trace context (new trace-id and span-id).
    pub fn new_root() -> Self {
        let hi = pseudo_random_u64();
        let lo = pseudo_random_u64();
        TraceContext {
            trace_id: format!("{:016x}{:016x}", hi, lo),
            span_id: format!("{:016x}", pseudo_random_u64()),
            flags: 0x01,
        }
    }

    /// Derive a child span that shares this context's `trace_id` but carries
    /// a freshly generated `span_id`.
    pub fn new_child_span(&self) -> Self {
        TraceContext {
            trace_id: self.trace_id.clone(),
            span_id: format!("{:016x}", pseudo_random_u64()),
            flags: self.flags,
        }
    }
}

/// Axum middleware that extracts (or synthesises) a W3C trace context,
/// injects it into request extensions, calls the next handler, and then
/// attaches the `traceparent` header to the response.
///
/// # Example
///
/// ```rust,ignore
/// use axum::middleware;
/// use fly_common::telemetry::trace_context_middleware;
///
/// let app = Router::new()
///     .route("/", get(handler))
///     .layer(middleware::from_fn(trace_context_middleware));
/// ```
pub async fn trace_context_middleware(mut req: Request<axum::body::Body>, next: Next) -> Response {
    let ctx = req
        .headers()
        .get("traceparent")
        .and_then(|v| v.to_str().ok())
        .and_then(TraceContext::from_header)
        .unwrap_or_else(TraceContext::new_root);

    tracing::debug!(trace_id = %ctx.trace_id, span_id = %ctx.span_id, "request trace");

    req.extensions_mut().insert(ctx.clone());

    let mut response = next.run(req).await;

    // Propagate child span in the response
    let child = ctx.new_child_span();
    if let Ok(value) = HeaderValue::from_str(&child.to_header()) {
        response.headers_mut().insert("traceparent", value);
    }

    response
}

/// Axum [extractor](https://docs.rs/axum/latest/axum/extract/index.html) that
/// retrieves the [`TraceContext`] inserted by [`trace_context_middleware`].
///
/// If the middleware is not present in the stack a synthetic root context is
/// returned instead of failing the request.
///
/// # Example
///
/// ```rust,ignore
/// async fn handler(TraceContextExt(ctx): TraceContextExt) -> String {
///     ctx.trace_id
/// }
/// ```
pub struct TraceContextExt(pub TraceContext);

#[axum::async_trait]
impl<S> FromRequestParts<S> for TraceContextExt
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let ctx = parts
            .extensions
            .get::<TraceContext>()
            .cloned()
            .unwrap_or_else(TraceContext::new_root);
        Ok(TraceContextExt(ctx))
    }
}
