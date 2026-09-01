# 🪽 fly-common

[![CI](https://github.com/radmuffin/fly-common/actions/workflows/ci.yml/badge.svg)](https://github.com/radmuffin/fly-common/actions/workflows/ci.yml)
[![Tests](https://img.shields.io/badge/tests-14%20passed-brightgreen.svg)](https://github.com/radmuffin/fly-common/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE)

> Lightweight, high-performance, reusable application base for Fly.io Axum + SQLite web applications.

`fly-common` provides the foundational building blocks to spin up production-ready, zero-cloud-cost web applications on Fly.io in minutes.

---

## ✨ Features

- **🚀 Axum + FlyServer**: Preconfigured port binding, graceful shutdown (`SIGTERM`/`SIGINT`), and `/healthz` machine health checks.
- **🛡️ SSRF & Security Shield**: Hardened outbound HTTP client, RFC-1918 / cloud-metadata blocking, and automatic security headers (`nosniff`, `SAMEORIGIN`, `strict-origin-when-cross-origin`).
- **🗄️ Fly SQLite (`FlyDb`)**: Automatic Write-Ahead Logging (WAL) configuration, foreign keys, synchronous mode, and migration helpers.
- **🔑 Anonymous Device Token (`UserToken`)**: Zero-friction user onboarding using cryptographic device tokens (`X-User-Token`) for instant state persistence without passwords.
- **👤 User Profile & Collaborator Models**: Generic `UserProfile`, `UpdateUserProfileRequest`, and `CollaboratorProfile` models for multi-device sync and real-time collaboration.
- **🎨 Frontend Shell (Vanilla ES6)**: Zero-build UI helpers (`FlyToast`, `FlyTheme`, `FlyClient`) and design tokens.
- **⚡ Reusable GitHub Workflows & CI**: Standardized CI workflow with automated `cargo test`, `clippy`, and formatting gates.

---

## 📦 Usage

### 1. Add Dependency to `Cargo.toml`

```toml
[dependencies]
fly_common = { git = "https://github.com/radmuffin/fly-common", tag = "v0.1.0" }
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

### 3. Frontend Example

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
