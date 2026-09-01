# 🪽 fly-common

[![CI](https://github.com/radmuffin/fly-common/actions/workflows/ci.yml/badge.svg)](https://github.com/radmuffin/fly-common/actions/workflows/ci.yml)
[![Tests](https://img.shields.io/badge/tests-14%20passed-brightgreen.svg)](https://github.com/radmuffin/fly-common/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE)

> Lightweight, high-performance, reusable application base for Fly.io Axum + SQLite web applications.

`fly-common` provides the foundational building blocks to spin up production-ready, zero-cloud-cost web applications on Fly.io in minutes.

---

## ✨ Features & Modular Tooling

`fly-common` is designed as a modular Swiss Army knife. The core stays minimal, while optional tools can be enabled via Cargo feature flags:

- **🚀 Axum + FlyServer**: Preconfigured port binding, graceful shutdown (`SIGTERM`/`SIGINT`), and `/healthz` machine health checks.
- **🛡️ SSRF & Security Shield**: Hardened outbound HTTP client, RFC-1918 / cloud-metadata blocking, and automatic security headers (`nosniff`, `SAMEORIGIN`, `strict-origin-when-cross-origin`).
- **🗄️ Fly SQLite (`FlyDb`)**: Automatic Write-Ahead Logging (WAL) configuration, foreign keys, synchronous mode, and migration helpers.
- **🔑 Anonymous Device Token (`UserToken`)**: Zero-friction user onboarding using cryptographic device tokens (`X-User-Token`) for instant state persistence without passwords.
- **👤 User Profile & Collaborators**: Generic `UserProfile`, `UpdateUserProfileRequest`, and `CollaboratorProfile` models for multi-device sync.
- **📱 Zero-Dependency SVG QR Code (`fly_common::qr`)**: Fast, pure Rust mathematical SVG QR code generation for pairing links and mobile sync.
- **🔍 Link Metadata & OpenGraph (`features = ["scraper"]`)**: Deterministic OpenGraph, Twitter Card, and JSON-LD microdata scraper with Send-safe tree dropping.
- **⚡ Real-Time Collaboration Hub (`features = ["ws"]`)**: Lightweight WebSocket pub/sub broadcasting hub for multi-user collaboration rooms and live state updates.
- **👥 Document & List Sharing Protocol (`features = ["sync"]`)**: UUIDv4 share token generators and collaborative resource schemas.
- **🗂️ Universal CSV & Batch Streamer (`features = ["io"]`)**: Streaming CSV deserializer and SQLite transaction chunking.
- **📊 Prometheus Metrics (`features = ["metrics"]`)**: Counter/gauge/histogram registration and OpenMetrics exposition via `/metrics`.
- **⏱️ Token Bucket Rate Limiter (`features = ["rate_limit"]`)**: Per-key rate limiting with `X-RateLimit-*` headers and `429` responses.
- **📡 SSE Broadcast Hub (`features = ["sse"]`)**: Named-room Server-Sent Events broadcaster for real-time browser push.
- **🔭 W3C Trace Context (`features = ["telemetry"]`)**: Pure-Rust traceparent header parser, propagator, and Axum middleware for distributed tracing — no heavy OpenTelemetry SDK required.
- **🎨 Frontend Shell (Vanilla ES6)**: Zero-build UI helpers (`FlyToast`, `FlyTheme`, `FlyClient`) and design tokens.

---

## ⚙️ Feature Flags

| Flag         | Default | Enables |
|--------------|---------|---------|
| `db`         | ✅      | `FlyDb` SQLite connection builder and migration runner |
| `auth`       | ✅      | `UserToken` extractor |
| `security`   | ✅      | SSRF validator, hardened HTTP client, security-header middleware, CORS layer |
| `server`     | ✅      | `FlyServer` builder, health endpoints, embedded static assets |
| `qr`         | ✅      | Pure-Rust SVG QR code generator |
| `scraper`    | ❌      | OpenGraph/Twitter Card/JSON-LD metadata scraper |
| `ws`         | ❌      | WebSocket broadcast hub |
| `sync`       | ❌      | UUIDv4 share-token and user-token generators |
| `io`         | ❌      | Streaming CSV deserializer and SQLite batch chunker |
| `metrics`    | ❌      | Prometheus-compatible `/metrics` endpoint |
| `rate_limit` | ❌      | Per-key token-bucket rate limiter and Axum middleware |
| `sse`        | ❌      | SSE broadcast hub and Axum handler helpers |
| `telemetry`  | ❌      | W3C traceparent propagation, middleware, and extractor |
| `full`       | ❌      | All of the above combined |

---

## 📦 Usage

### 1. Add Dependency to `Cargo.toml`

```toml
[dependencies]
# Minimal Core
fly_common = { git = "https://github.com/radmuffin/fly-common" }

# Or with specific modular features:
# fly_common = { git = "https://github.com/radmuffin/fly-common", features = ["ws", "scraper", "sync", "io"] }

# Everything enabled:
# fly_common = { git = "https://github.com/radmuffin/fly-common", features = ["full"] }
# i.e. includes scraper, ws, sync, io, metrics, rate_limit, sse, telemetry
```

### 2. Backend Example

```rust
use axum::{routing::get, Json, Router};
use fly_common::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Initialize SQLite connection with WAL mode
    let db = FlyDb::open_shared("app.db")?;

    // 2. Define custom app API
    let api = Router::new()
        .route("/items", get(list_items));

    // 3. Launch FlyServer
    FlyServer::builder()
        .with_app_info("My App", "0.1.0")
        .nest("/api", api)
        .with_static_dir("static")
        .serve()
        .await
}

async fn list_items(user: UserToken) -> Json<ApiResponse<Vec<String>>> {
    println!("Request from device token: {}", user);
    Json(ApiResponse::ok(vec!["Item 1".into(), "Item 2".into()]))
}
```

### 3. Distributed Tracing Example (`features = ["telemetry"]`)

```rust
use axum::{middleware, routing::get, Router};
use fly_common::telemetry::{trace_context_middleware, TraceContextExt};

let app = Router::new()
    .route("/", get(handler))
    .layer(middleware::from_fn(trace_context_middleware));

async fn handler(TraceContextExt(ctx): TraceContextExt) -> String {
    format!("trace_id={}", ctx.trace_id)
}
```

### 4. Frontend Example

```html
<link rel="stylesheet" href="/_fly/fly-base.css">

<script type="module">
  import { FlyClient } from '/_fly/fly-device-sync.js';
  import { FlyToast } from '/_fly/fly-ui.js';

  const client = new FlyClient({ baseUrl: '/api' });
  const data = await client.get('/items');
  FlyToast.success('Loaded items!');
</script>
```

---

## 📜 License

MIT OR Apache-2.0
