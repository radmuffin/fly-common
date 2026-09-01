use axum::{
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};

/// Standard API response wrapper for JSON endpoints.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApiResponse<T> {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl<T> ApiResponse<T> {
    pub fn ok(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            message: None,
        }
    }

    pub fn err(error: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error.into()),
            message: None,
        }
    }

    pub fn message(msg: impl Into<String>) -> Self {
        Self {
            success: true,
            data: None,
            error: None,
            message: Some(msg.into()),
        }
    }
}

impl<T: Serialize> IntoResponse for ApiResponse<T> {
    fn into_response(self) -> Response {
        Json(self).into_response()
    }
}

/// Basic application metadata payload (e.g. for /api/info).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppInfo {
    pub name: String,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
}

/// Anonymous user profile for multi-device sync and shared sessions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct UserProfile {
    pub user_token: String,
    pub name: String,
    pub avatar: String,
    pub color: String,
}

/// Request payload to update an anonymous user profile.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateUserProfileRequest {
    pub name: Option<String>,
    pub avatar: Option<String>,
    pub color: Option<String>,
}

/// Collaborator representation for shared documents, collections, or trips.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct CollaboratorProfile {
    pub name: String,
    pub avatar: String,
    pub color: String,
    pub is_owner: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_response_serialization() {
        let ok_res = ApiResponse::ok("hello");
        assert!(ok_res.success);
        assert_eq!(ok_res.data.as_deref(), Some("hello"));

        let err_res: ApiResponse<()> = ApiResponse::err("not found");
        assert!(!err_res.success);
        assert_eq!(err_res.error.as_deref(), Some("not found"));
    }
}
