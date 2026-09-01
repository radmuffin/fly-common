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

---

## ⚠️ Critical Constraints & Rules

1. **Keep dependencies lean**: `fly_common` is shared across all downstream apps; do not add heavy or application-specific dependencies.
2. **Backward Compatibility**: Ensure public interfaces (`FlyServer`, `FlyDb`, `UserToken`, `ApiResponse`) remain backward compatible across minor versions.
3. **No Unsafe Futures**: Ensure all Axum handlers and extractors implement `Send + Sync`.
