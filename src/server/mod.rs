use axum::{
    extract::DefaultBodyLimit,
    http::header::{CACHE_CONTROL, CONTENT_TYPE},
    middleware,
    response::{IntoResponse, Json, Response},
    routing::get,
    Router,
};
use serde_json::{json, Value};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use tower_http::services::{ServeDir, ServeFile};

use crate::models::AppInfo;
use crate::security::{set_security_headers, standard_cors_layer};

/// Axum Server builder tailored for Fly.io deployments.
pub struct FlyServer {
    name: String,
    version: String,
    port: u16,
    body_limit_bytes: usize,
    static_dir: Option<PathBuf>,
    app_router: Router,
    app_info: Option<AppInfo>,
}

impl FlyServer {
    /// Initializes a new FlyServer builder with sensible defaults.
    pub fn builder() -> Self {
        let port: u16 = std::env::var("PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(3000);

        Self {
            name: "fly-app".into(),
            version: "0.1.0".into(),
            port,
            body_limit_bytes: 2 * 1024 * 1024, // 2MB
            static_dir: None,
            app_router: Router::new(),
            app_info: None,
        }
    }

    /// Sets the application name and version.
    pub fn with_app_info(mut self, name: impl Into<String>, version: impl Into<String>) -> Self {
        let name_str = name.into();
        let ver_str = version.into();
        self.name = name_str.clone();
        self.version = ver_str.clone();
        self.app_info = Some(AppInfo {
            name: name_str,
            version: ver_str,
            repository: None,
            license: Some("MIT".into()),
        });
        self
    }

    /// Overrides listening port.
    pub fn with_port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    /// Sets the request body limit in bytes.
    pub fn with_body_limit(mut self, bytes: usize) -> Self {
        self.body_limit_bytes = bytes;
        self
    }

    /// Enables static file serving with single-page-app (SPA) fallback to index.html.
    pub fn with_static_dir<P: AsRef<Path>>(mut self, dir: P) -> Self {
        self.static_dir = Some(dir.as_ref().to_path_buf());
        self
    }

    /// Merges application API router.
    pub fn with_routes(mut self, router: Router) -> Self {
        self.app_router = self.app_router.merge(router);
        self
    }

    /// Nests application API router under a path prefix (e.g. "/api").
    pub fn nest(mut self, path: &str, router: Router) -> Self {
        self.app_router = self.app_router.nest(path, router);
        self
    }

    /// Builds the complete Axum Router.
    pub fn build_router(self) -> Router {
        let info = self.app_info.unwrap_or_else(|| AppInfo {
            name: self.name.clone(),
            version: self.version.clone(),
            repository: None,
            license: Some("MIT".into()),
        });

        // Embedded base frontend assets under /_fly/*
        let embedded_static = Router::new()
            .route("/_fly/fly-base.css", get(serve_fly_css))
            .route("/_fly/fly-ui.js", get(serve_fly_ui_js))
            .route("/_fly/fly-device-sync.js", get(serve_fly_device_sync_js));

        // Base health routes for Fly.io machine health checks (/health, /healthz, /up)
        let mut base_router = Router::new()
            .route("/health", get(health_check))
            .route("/healthz", get(health_check))
            .route("/up", get(health_check))
            .route("/api/info", get(move || {
                let info_clone = info.clone();
                async move { Json(info_clone) }
            }))
            .merge(embedded_static);

        base_router = base_router.merge(self.app_router);

        if let Some(static_path) = self.static_dir {
            let index_path = static_path.join("index.html");
            let static_service = ServeDir::new(&static_path)
                .fallback(ServeFile::new(index_path));
            base_router = base_router.fallback_service(static_service);
        }

        base_router
            .layer(middleware::from_fn(set_security_headers))
            .layer(standard_cors_layer())
            .layer(DefaultBodyLimit::max(self.body_limit_bytes))
    }

    /// Starts the server and listens for incoming connections and graceful shutdown signals.
    pub async fn serve(self) -> Result<(), Box<dyn std::error::Error>> {
        let port = self.port;
        let name = self.name.clone();
        let app = self.build_router();

        let addr = SocketAddr::from(([0, 0, 0, 0], port));
        println!("🚀 [{}] Server listening on http://0.0.0.0:{}", name, port);

        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, app)
            .with_graceful_shutdown(shutdown_signal())
            .await?;

        Ok(())
    }
}

/// Standard JSON healthcheck handler.
async fn health_check() -> Json<Value> {
    Json(json!({
        "status": "ok",
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

// Embedded static asset handlers
async fn serve_fly_css() -> Response {
    (
        [
            (CONTENT_TYPE, "text/css; charset=utf-8"),
            (CACHE_CONTROL, "public, max-age=86400"),
        ],
        include_str!("../../static/fly-base.css"),
    )
        .into_response()
}

async fn serve_fly_ui_js() -> Response {
    (
        [
            (CONTENT_TYPE, "application/javascript; charset=utf-8"),
            (CACHE_CONTROL, "public, max-age=86400"),
        ],
        include_str!("../../static/fly-ui.js"),
    )
        .into_response()
}

async fn serve_fly_device_sync_js() -> Response {
    (
        [
            (CONTENT_TYPE, "application/javascript; charset=utf-8"),
            (CACHE_CONTROL, "public, max-age=86400"),
        ],
        include_str!("../../static/fly-device-sync.js"),
    )
        .into_response()
}

/// Listens for SIGINT (Ctrl+C) and SIGTERM (Fly.io machine termination).
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            println!("🛑 Received SIGINT signal, initiating graceful shutdown...");
        },
        _ = terminate => {
            println!("🛑 Received SIGTERM signal (Fly.io stop), initiating graceful shutdown...");
        },
    }
}
