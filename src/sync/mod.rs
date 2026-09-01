use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Generates a random URL-safe UUIDv4 token for sharing collections, trips, or documents.
pub fn generate_share_token() -> String {
    Uuid::new_v4().simple().to_string()
}

/// Generates a random cryptographic user token with prefix.
pub fn generate_user_token(prefix: &str) -> String {
    format!("{}{}", prefix, Uuid::new_v4().simple())
}

/// Generic collaborator permissions record.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResourceCollaborator {
    pub user_token: String,
    pub resource_id: i64,
    pub is_owner: bool,
    pub joined_at: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_share_and_user_tokens() {
        let share_tok = generate_share_token();
        assert_eq!(share_tok.len(), 32);
        assert!(share_tok.chars().all(|c| c.is_ascii_hexdigit()));

        let user_tok = generate_user_token("usr_");
        assert!(user_tok.starts_with("usr_"));
        assert_eq!(user_tok.len(), 36);
    }
}
