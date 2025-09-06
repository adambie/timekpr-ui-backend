use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use chrono::{DateTime, Utc};

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