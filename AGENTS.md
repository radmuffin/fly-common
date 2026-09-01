# 🤖 AGENTS.md — AI Agent & Developer Guidance for `fly-common`

Welcome! This document outlines the architecture, design constraints, and module guidelines for **`fly-common`**, the reusable application base for Fly.io Axum + SQLite web applications.

---

## 🎯 Project Goals

- **Zero-Cloud-Cost & Lightweight**: Embedded SQLite (WAL mode) designed for sub-millisecond responses on single-instance Fly.io machines with persistent volumes.
- **Anonymous Device Identity**: Passwordless user persistence using client-generated cryptographic device tokens (`x-user-token`).
- **Zero-Build Frontend**: Vanilla ES6 modules and modern CSS design tokens served directly or embedded without requiring Vite, Webpack, or Node runtime dependencies.
- **SSRF & Defensive Security**: Outbound HTTP validation strictly blocking RFC 1918 private IPv4/IPv6, cloud metadata addresses (AWS/GCP/Azure `169.254.169.254`), and internal hostnames.

---

## 🏛️ Module Architecture

1. **`fly_common::security`**:
   - `ssrf`: IP parsing and DNS validation for blocking private targets.
   - `client`: Hardened `reqwest::Client` with custom redirect policy that inspects every redirect target for SSRF.
   - `headers`: Middleware injecting `nosniff`, `SAMEORIGIN`, and `strict-origin-when-cross-origin`.
   - `cors`: Permissive CORS allowing `x-user-token` header.

2. **`fly_common::auth`**:
   - `UserToken`: Axum `FromRequestParts` extractor parsing `x-user-token` header and query parameters with sanitization.

3. **`fly_common::db`**:
   - `FlyDb`: SQLite connection builder that applies `PRAGMA journal_mode = WAL;`, `foreign_keys = ON;`, `synchronous = NORMAL;`, and `busy_timeout = 5000;`.
   - Migration runner for applying batch SQL migrations safely inside transactions.

4. **`fly_common::server`**:
   - `FlyServer`: Axum server builder integrating `/health`, `/healthz`, `/up`, `/api/info`, graceful shutdown signal (`SIGTERM`/`SIGINT`), and static SPA serving with embedded `/_fly/*` assets.

5. **`fly_common::models`**:
   - `ApiResponse<T>`: Standardized JSON envelope (`success`, `data`, `error`, `message`).
   - `AppInfo`: Metadata payload for `/api/info`.

6. **`fly_common::metrics`** *(feature: `metrics`)*:
   - Prometheus-compatible counter/gauge/histogram registration and exposition via a `/metrics` HTTP endpoint.
   - All metric families are registered at startup and serialized in the OpenMetrics text format.

7. **`fly_common::rate_limit`** *(feature: `rate_limit`)*:
   - Token bucket rate limiter with per-key (e.g. per-device-token) capacity and refill intervals.
   - Axum middleware that injects `X-RateLimit-*` response headers and returns `429 Too Many Requests` on exhaustion.

8. **`fly_common::sse`** *(feature: `sse`)*:
   - Server-Sent Events (SSE) broadcast hub for real-time push to multiple browser clients.
   - `SseBroadcastHub`: typed channel wrapper that maps named rooms to `tokio::sync::broadcast` senders.
   - Axum handler helper that streams events to connected clients using `axum::response::Sse`.

9. **`fly_common::telemetry`** *(feature: `telemetry`)*:
   - Lightweight W3C Trace Context (traceparent) header parser, propagator, and Axum middleware.
   - Zero heavy dependencies — implemented in pure Rust using `std::time` and `std::sync::atomic`.
   - `TraceContext`: parsed trace-id / span-id / flags struct with `from_header()`, `to_header()`, `new_root()`, `new_child_span()`.
   - `trace_context_middleware`: Axum `from_fn` middleware that extracts or synthesises a context, injects it into request extensions, and echoes it in the response `traceparent` header.
   - `TraceContextExt`: Axum `FromRequestParts` extractor for downstream handlers.

---

## ⚙️ Feature Flags

| Flag         | Default | Enables |
|--------------|---------|---------|
| `db`         | ✅      | `FlyDb` SQLite connection builder and migration runner |
| `auth`       | ✅      | `UserToken` extractor |
| `security`   | ✅      | SSRF validator, hardened HTTP client, security-header middleware, CORS layer |
| `server`     | ✅      | `FlyServer` builder, health endpoints, embedded static assets |
| `qr`         | ✅      | Pure-Rust SVG QR code generator |
| `scraper`    | ❌      | OpenGraph/Twitter Card/JSON-LD metadata scraper (`scraper` crate) |
| `ws`         | ❌      | WebSocket broadcast hub (`axum/ws`, `futures-util`) |
| `sync`       | ❌      | UUIDv4 share-token and user-token generators (`uuid` crate) |
| `io`         | ❌      | Streaming CSV deserializer and SQLite batch chunker (`csv` crate) |
| `metrics`    | ❌      | Prometheus-compatible `/metrics` endpoint |
| `rate_limit` | ❌      | Per-key token-bucket rate limiter and Axum middleware |
| `sse`        | ❌      | SSE broadcast hub and Axum handler helpers |
| `telemetry`  | ❌      | W3C traceparent propagation, middleware, and extractor |
| `full`       | ❌      | All of the above combined |

---

## ⚠️ Critical Constraints & Rules

1. **Keep dependencies lean**: `fly_common` is shared across all downstream apps; do not add heavy or application-specific dependencies.
2. **Backward Compatibility**: Ensure public interfaces (`FlyServer`, `FlyDb`, `UserToken`, `ApiResponse`) remain backward compatible across minor versions.
3. **No Unsafe Futures**: Ensure all Axum handlers and extractors implement `Send + Sync`.
4. **WAL Checkpoint Awareness**: The SQLite connection builder enables WAL mode. Long-lived read transactions can stall WAL checkpoints and cause unbounded growth of the `-wal` file. Keep transaction lifetimes short and avoid holding the `Arc<Mutex<Connection>>` lock across `.await` points.
5. **SSRF on Every Outbound Request**: Any new code that makes outbound HTTP calls **must** call `validate_url_for_ssrf()` and use `build_safe_http_client()` — no exceptions.
