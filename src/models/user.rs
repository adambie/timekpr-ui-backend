use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Database entity representing a managed user
#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize, ToSchema)]
pub struct ManagedUser {
    pub id: i64,
    pub username: String,
    pub system_ip: String,
    pub is_valid: bool,
    pub date_added: Option<DateTime<Utc>>,
    pub last_checked: Option<DateTime<Utc>>,
    pub last_config: Option<String>,
    pub pending_time_adjustment: Option<i64>,
    pub pending_time_operation: Option<String>,
}

/// Business model for time modifications
#[derive(Debug, Clone)]
pub struct TimeModification {
    pub user_id: i64,
    pub operation: String, // "+" or "-"
    pub seconds: i64,
}

impl TimeModification {
    pub fn new(user_id: i64, operation: String, seconds: i64) -> Result<Self, String> {
        if operation != "+" && operation != "-" {
            return Err("Operation must be '+' or '-'".to_string());
        }

        if seconds <= 0 {
            return Err("Seconds must be positive".to_string());
        }

        Ok(Self {
            user_id,
            operation,
            seconds,
        })
    }
}
