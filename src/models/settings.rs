use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Database entity representing a settings entry
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SettingsEntry {
    pub id: i64,
    pub key: String,
    pub value: String,
}

impl SettingsEntry {
    /// Create a new settings entry (for insertion, ID will be auto-generated)
    pub fn new(key: String, value: String) -> Self {
        Self {
            id: 0, // Will be set by database on insert
            key,
            value,
        }
    }

    /// Create settings entry with existing ID (for updates/retrieval)
    pub fn with_id(id: i64, key: String, value: String) -> Self {
        Self { id, key, value }
    }


}

/// Helper constants for common setting keys
#[allow(dead_code)]
impl SettingsEntry {
    pub const ADMIN_PASSWORD_HASH: &'static str = "admin_password_hash";
    pub const JWT_SECRET: &'static str = "jwt_secret";
    pub const CHECK_INTERVAL: &'static str = "check_interval";
}
