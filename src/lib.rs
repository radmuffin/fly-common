pub mod auth;
pub mod db;
pub mod models;
pub mod security;
pub mod server;

pub mod prelude {
    pub use crate::auth::UserToken;
    pub use crate::db::{DbPool, FlyDb};
    pub use crate::models::{
        ApiResponse, AppInfo, CollaboratorProfile, UpdateUserProfileRequest, UserProfile,
    };
    pub use crate::security::{
        build_safe_http_client, is_private_or_restricted_ip, is_restricted_hostname,
        set_security_headers, standard_cors_layer, validate_parsed_url, validate_url_for_ssrf,
    };
    pub use crate::server::FlyServer;
}
