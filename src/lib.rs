pub mod auth;
pub mod db;
pub mod models;
pub mod qr;
pub mod security;
pub mod server;

#[cfg(feature = "scraper")]
pub mod scraper;

#[cfg(feature = "ws")]
pub mod ws;

#[cfg(feature = "sync")]
pub mod sync;

#[cfg(feature = "io")]
pub mod io;

#[cfg(feature = "sse")]
pub mod sse;

#[cfg(feature = "metrics")]
pub mod metrics;

#[cfg(feature = "rate-limit")]
pub mod rate_limit;

#[cfg(feature = "telemetry")]
pub mod telemetry;

pub mod prelude {
    pub use crate::auth::UserToken;
    pub use crate::db::{DbPool, FlyDb};
    pub use crate::models::{
        ApiResponse, AppInfo, CollaboratorProfile, UpdateUserProfileRequest, UserProfile,
    };
    pub use crate::qr::{generate_qr_svg, QrSvgResult};
    pub use crate::security::{
        build_safe_http_client, is_private_or_restricted_ip, is_restricted_hostname,
        set_security_headers, standard_cors_layer, validate_parsed_url, validate_url_for_ssrf,
        validate_url_for_ssrf_async,
    };
    pub use crate::server::FlyServer;

    #[cfg(feature = "scraper")]
    pub use crate::scraper::{scrape_page_metadata, PageMetadata};

    #[cfg(feature = "ws")]
    pub use crate::ws::{BroadcastHub, WsMessage};

    #[cfg(feature = "sync")]
    pub use crate::sync::{generate_share_token, generate_user_token, ResourceCollaborator};

    #[cfg(feature = "io")]
    pub use crate::io::{chunk_slice, parse_csv};

    #[cfg(feature = "sse")]
    pub use crate::sse::{SseBroadcastHub, SseMessage};

    #[cfg(feature = "metrics")]
    pub use crate::metrics::{metrics_handler, FlyMetrics};

    #[cfg(feature = "rate-limit")]
    pub use crate::rate_limit::{RateLimitResult, RateLimiter};

    #[cfg(feature = "telemetry")]
    pub use crate::telemetry::{trace_context_middleware, TraceContext, TraceContextExt};
}
