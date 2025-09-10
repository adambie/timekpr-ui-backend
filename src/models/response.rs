use serde::Serialize;
use utoipa::ToSchema;

// Common response types
#[derive(Serialize, ToSchema)]
pub struct ApiResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Serialize, ToSchema)]
pub struct ErrorResponse {
    pub success: bool,
    pub message: String,
}

// Authentication responses
#[derive(Serialize, ToSchema)]
pub struct LoginResponse {
    pub success: bool,
    pub message: String,
    pub token: String,
    pub expires_in: u64,  // seconds
}

// User management responses
#[derive(Serialize, ToSchema)]
pub struct UserData {
    pub id: i64,
    pub username: String,
    pub system_ip: String,
    pub time_left: String,
    pub last_checked: String,
    pub pending_adjustment: Option<String>,
    pub pending_schedule: bool,
}

#[derive(Serialize, ToSchema)]
pub struct DashboardResponse {
    pub success: bool,
    pub users: Vec<UserData>,
}

#[derive(Serialize, ToSchema)]
pub struct AdminUserData {
    pub id: i64,
    pub username: String,
    pub system_ip: String,
    pub is_valid: bool,
    pub last_checked: String,
}

#[derive(Serialize, ToSchema)]
pub struct AdminResponse {
    pub success: bool,
    pub users: Vec<AdminUserData>,
}

#[derive(Serialize, ToSchema)]
pub struct ModifyTimeResponse {
    pub success: bool,
    pub message: String,
    pub username: String,
    pub refresh: Option<bool>,
    pub pending: Option<bool>,
}

// Usage tracking responses
#[derive(Serialize, ToSchema)]
pub struct UsageData {
    pub date: String,
    pub hours: f64,
}

#[derive(Serialize, ToSchema)]
pub struct UsageResponse {
    pub success: bool,
    pub data: Vec<UsageData>,
    pub username: String,
}

// Schedule management responses - using domain models directly
use crate::models::{WeeklyHours, WeeklyTimeIntervals};

#[derive(Serialize, ToSchema)]
pub struct ScheduleWithIntervals {
    pub hours: WeeklyHours,
    pub intervals: WeeklyTimeIntervals,
}

#[derive(Serialize, ToSchema)]
pub struct ScheduleSyncResponse {
    pub success: bool,
    pub is_synced: bool,
    pub schedule: Option<ScheduleWithIntervals>,
    pub last_synced: Option<String>,
    pub last_modified: Option<String>,
}

// Service status type (used by service layer)
#[derive(Serialize)]
pub struct ScheduleSyncStatus {
    pub is_synced: bool,
    pub schedule: Option<ScheduleWithIntervals>,
    pub last_synced: Option<String>,
    pub last_modified: Option<String>,
}

// System status responses
#[derive(Serialize, ToSchema)]
pub struct TaskStatusData {
    pub running: bool,
    pub last_update: String,
    pub managed_users: i64,
}

#[derive(Serialize, ToSchema)]
pub struct TaskStatusResponse {
    pub success: bool,
    pub status: TaskStatusData,
}

#[derive(Serialize, ToSchema)]
pub struct SshStatusResponse {
    pub success: bool,
    pub ssh_key_exists: bool,
    pub message: String,
}