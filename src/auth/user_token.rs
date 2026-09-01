use axum::{async_trait, extract::FromRequestParts, http::request::Parts};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::Deref;

/// Represents an anonymous device or user token extracted from incoming requests.
/// Looks in `x-user-token` header first, then query parameters (`token` or `user_token`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UserToken(String);

impl UserToken {
    /// Creates a new UserToken after sanitization.
    pub fn new(token: impl Into<String>) -> Self {
        let raw = token.into();
        let sanitized: String = raw
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
            .take(128)
            .collect();
        UserToken(sanitized)
    }

    /// Returns the underlying token string.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns true if the token is non-empty.
    pub fn is_present(&self) -> bool {
        !self.0.is_empty()
    }
}

impl Deref for UserToken {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl fmt::Display for UserToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for UserToken
where
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // 1. Check x-user-token header
        if let Some(header_val) = parts.headers.get("x-user-token") {
            if let Ok(token_str) = header_val.to_str() {
                if !token_str.trim().is_empty() {
                    return Ok(UserToken::new(token_str));
                }
            }
        }

        // 2. Check query params if header missing
        if let Some(query) = parts.uri.query() {
            for pair in query.split('&') {
                let mut it = pair.split('=');
                if let (Some(k), Some(v)) = (it.next(), it.next()) {
                    if (k == "token" || k == "user_token" || k == "x-user-token")
                        && !v.trim().is_empty()
                    {
                        return Ok(UserToken::new(v));
                    }
                }
            }
        }

        Ok(UserToken(String::new()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_token_sanitization() {
        let token = UserToken::new("abc-123_XYZ!@#$");
        assert_eq!(token.as_str(), "abc-123_XYZ");
        assert!(token.is_present());

        let empty = UserToken::new("!@#$");
        assert_eq!(empty.as_str(), "");
        assert!(!empty.is_present());
    }
}
