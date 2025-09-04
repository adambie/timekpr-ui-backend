use axum::{
    extract::{Path, Query, State},
    response::Json,
    Form,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tower_sessions::Session;
use std::collections::HashMap;

use crate::{
    database::models::{ManagedUser, UserDailyTimeInterval, UserWeeklySchedule},
    services::ssh::SSHClient,
    ssh::SSHClient as LegacySSHClient,
    utils::auth::is_authenticated,
    AppState,
};

#[derive(Serialize)]
pub struct ApiResponse<T> {
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
    #[serde(flatten)]
    data: Option<T>,
}

impl<T> ApiResponse<T> {
    fn success(data: T) -> Self {
        Self { success: true, message: None, data: Some(data) }
    }
    
    fn error(message: String) -> Self {
        Self { success: false, message: Some(message), data: None }
    }
}

#[derive(Deserialize)]
pub struct UsageQuery {
    days: Option<i32>,
}

#[derive(Deserialize)]
pub struct ModifyTimeForm {
    user_id: i64,
    operation: String,
    seconds: i64,
}

#[derive(Serialize)]
pub struct TaskStatus {
    running: bool,
    thread_alive: bool,
    last_error: Option<String>,
}

pub async fn task_status(State(state): State<AppState>, session: Session) -> Json<Value> {
    if !is_authenticated(&session).await {
        return Json(json!({"success": false, "message": "Not authenticated"}));
    }
    
    Json(json!({
        "success": true,
        "status": {
            "running": state.scheduler.is_running().await,
            "thread_alive": state.scheduler.is_running().await,
            "last_error": null
        }
    }))
}

pub async fn restart_tasks(State(state): State<AppState>, session: Session) -> Json<Value> {
    if !is_authenticated(&session).await {
        return Json(json!({"success": false, "message": "Not authenticated"}));
    }
    
    state.scheduler.stop().await;
    state.scheduler.start().await;
    
    Json(json!({"success": true, "message": "Background tasks restarted"}))
}


pub async fn get_user_usage(
    State(state): State<AppState>,
    session: Session,
    Path(user_id): Path<i64>,
    Query(params): Query<UsageQuery>,
) -> Json<Value> {
    if !is_authenticated(&session).await {
        return Json(json!({"success": false, "message": "Not authenticated"}));
    }
    
    let days = params.days.unwrap_or(7);
    
    match sqlx::query_as::<_, ManagedUser>(
        "SELECT id, username, system_ip, is_valid, date_added, last_checked, last_config, pending_time_adjustment, pending_time_operation FROM managed_user WHERE id = ?"
    )
    .bind(user_id)
    .fetch_optional(&state.pool)
    .await {
        Ok(Some(user)) => {
            match user.get_recent_usage(&state.pool, days).await {
                Ok(usage_data) => {
                    let labels: Vec<String> = usage_data.keys().cloned().collect();
                    let values: Vec<f64> = usage_data.values().map(|&v| (v as f64) / 3600.0).collect();
                    
                    Json(json!({
                        "success": true,
                        "labels": labels,
                        "values": values,
                        "username": user.username
                    }))
                }
                Err(e) => {
                    tracing::error!("Failed to get usage data: {}", e);
                    Json(json!({"success": false, "message": "Failed to get usage data"}))
                }
            }
        }
        Ok(None) => Json(json!({"success": false, "message": "User not found"})),
        Err(e) => {
            tracing::error!("Database error: {}", e);
            Json(json!({"success": false, "message": "Database error"}))
        }
    }
}

pub async fn modify_time(
    State(state): State<AppState>,
    session: Session,
    Form(form): Form<ModifyTimeForm>,
) -> Json<Value> {
    if !is_authenticated(&session).await {
        return Json(json!({"success": false, "message": "Not authenticated"}));
    }
    
    if !matches!(form.operation.as_str(), "+" | "-") {
        return Json(json!({"success": false, "message": "Operation must be '+' or '-'"}));
    }
    
    match sqlx::query_as::<_, ManagedUser>(
        "SELECT id, username, system_ip, is_valid, date_added, last_checked, last_config, pending_time_adjustment, pending_time_operation FROM managed_user WHERE id = ?"
    )
    .bind(form.user_id)
    .fetch_optional(&state.pool)
    .await {
        Ok(Some(user)) => {
            let ssh_client = SSHClient::new(user.system_ip.clone());
            
            match ssh_client.modify_time_left(&user.username, &form.operation, form.seconds).await {
                Ok((true, message)) => {
                    // Success - clear any pending adjustments and update user info
                    let _ = ManagedUser::clear_pending_adjustment(&state.pool, user.id).await;
                    
                    // Update user config
                    if let Ok((is_valid, _, config)) = ssh_client.validate_user(&user.username).await {
                        if is_valid && config.is_some() {
                            let config_json = serde_json::to_string(&config.unwrap()).unwrap_or_default();
                            let _ = ManagedUser::update_validation(&state.pool, user.id, is_valid, Some(&config_json)).await;
                        }
                    }
                    
                    Json(json!({
                        "success": true,
                        "message": message,
                        "username": user.username,
                        "refresh": true
                    }))
                }
                Ok((false, _)) => {
                    // Failed - store as pending adjustment
                    let _ = ManagedUser::set_pending_adjustment(&state.pool, user.id, form.seconds, &form.operation).await;
                    
                    Json(json!({
                        "success": true,
                        "message": format!("Computer seems to be offline. Time adjustment of {}{} seconds has been queued and will be applied when the computer comes online.", form.operation, form.seconds),
                        "username": user.username,
                        "pending": true,
                        "refresh": true
                    }))
                }
                Err(e) => {
                    tracing::error!("SSH error: {}", e);
                    Json(json!({"success": false, "message": "SSH connection failed"}))
                }
            }
        }
        Ok(None) => Json(json!({"success": false, "message": "User not found"})),
        Err(e) => {
            tracing::error!("Database error: {}", e);
            Json(json!({"success": false, "message": "Database error"}))
        }
    }
}

pub async fn get_user_intervals(
    State(state): State<AppState>,
    session: Session,
    Path(user_id): Path<i64>,
) -> Json<Value> {
    if !is_authenticated(&session).await {
        return Json(json!({"success": false, "message": "Not authenticated"}));
    }
    
    match UserDailyTimeInterval::get_by_user_id(&state.pool, user_id).await {
        Ok(intervals) => {
            let mut intervals_dict = HashMap::new();
            
            for interval in intervals {
                intervals_dict.insert(interval.day_of_week.to_string(), json!({
                    "id": interval.id,
                    "day_name": interval.get_day_name(),
                    "start_hour": interval.start_hour,
                    "start_minute": interval.start_minute,
                    "end_hour": interval.end_hour,
                    "end_minute": interval.end_minute,
                    "is_enabled": interval.is_enabled,
                    "is_synced": interval.is_synced,
                    "time_range": interval.get_time_range_string(),
                    "last_synced": interval.last_synced.map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
                }));
            }
            
            // Get username
            let username = sqlx::query_scalar::<_, String>("SELECT username FROM managed_user WHERE id = ?")
                .bind(user_id)
                .fetch_optional(&state.pool)
                .await
                .unwrap_or_default()
                .unwrap_or_default();
            
            Json(json!({
                "success": true,
                "intervals": intervals_dict,
                "username": username
            }))
        }
        Err(e) => {
            tracing::error!("Database error: {}", e);
            Json(json!({"success": false, "message": "Database error"}))
        }
    }
}

pub async fn update_user_intervals(
    State(state): State<AppState>,
    session: Session,
    Path(user_id): Path<i64>,
    Json(data): Json<Value>,
) -> Json<Value> {
    if !is_authenticated(&session).await {
        return Json(json!({"success": false, "message": "Not authenticated"}));
    }
    
    if let Some(intervals_data) = data.get("intervals").and_then(|v| v.as_object()) {
        for (day_str, interval_data) in intervals_data {
            if let Ok(day_of_week) = day_str.parse::<i32>() {
                if !(1..=7).contains(&day_of_week) {
                    continue;
                }
                
                let start_hour = interval_data.get("start_hour").and_then(|v| v.as_i64()).unwrap_or(9) as i32;
                let start_minute = interval_data.get("start_minute").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                let end_hour = interval_data.get("end_hour").and_then(|v| v.as_i64()).unwrap_or(17) as i32;
                let end_minute = interval_data.get("end_minute").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                let is_enabled = interval_data.get("is_enabled").and_then(|v| v.as_bool()).unwrap_or(false);
                
                // Create temporary interval to validate
                let temp_interval = UserDailyTimeInterval {
                    id: 0,
                    user_id,
                    day_of_week,
                    start_hour,
                    start_minute,
                    end_hour,
                    end_minute,
                    is_enabled,
                    is_synced: false,
                    last_synced: None,
                    last_modified: chrono::Utc::now(),
                };
                
                if !temp_interval.is_valid_interval() {
                    return Json(json!({
                        "success": false,
                        "message": format!("Invalid time interval for {}: start time must be before end time", temp_interval.get_day_name())
                    }));
                }
                
                let _ = UserDailyTimeInterval::upsert(&state.pool, user_id, day_of_week, &temp_interval).await;
            }
        }
        
        let username = sqlx::query_scalar::<_, String>("SELECT username FROM managed_user WHERE id = ?")
            .bind(user_id)
            .fetch_optional(&state.pool)
            .await
            .unwrap_or_default()
            .unwrap_or_default();
        
        Json(json!({
            "success": true,
            "message": format!("Time intervals updated for {}", username),
            "username": username
        }))
    } else {
        Json(json!({"success": false, "message": "Invalid data format"}))
    }
}

pub async fn get_intervals_sync_status(
    State(state): State<AppState>,
    session: Session,
    Path(user_id): Path<i64>,
) -> Json<Value> {
    if !is_authenticated(&session).await {
        return Json(json!({"success": false, "message": "Not authenticated"}));
    }
    
    match UserDailyTimeInterval::get_by_user_id(&state.pool, user_id).await {
        Ok(intervals) => {
            let needs_sync = intervals.iter().any(|i| !i.is_synced);
            let last_synced = intervals.iter()
                .filter_map(|i| i.last_synced)
                .max()
                .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string());
            
            let enabled_count = intervals.iter().filter(|i| i.is_enabled).count();
            
            let username = sqlx::query_scalar::<_, String>("SELECT username FROM managed_user WHERE id = ?")
                .bind(user_id)
                .fetch_optional(&state.pool)
                .await
                .unwrap_or_default()
                .unwrap_or_default();
            
            Json(json!({
                "success": true,
                "needs_sync": needs_sync,
                "last_synced": last_synced,
                "enabled_intervals": enabled_count,
                "total_intervals": intervals.len(),
                "username": username
            }))
        }
        Err(e) => {
            tracing::error!("Database error: {}", e);
            Json(json!({"success": false, "message": "Database error"}))
        }
    }
}

pub async fn get_schedule_sync_status(
    State(state): State<AppState>,
    session: Session,
    Path(user_id): Path<i64>,
) -> Json<Value> {
    if !is_authenticated(&session).await {
        return Json(json!({"success": false, "message": "Not authenticated"}));
    }
    
    match UserWeeklySchedule::get_by_user_id(&state.pool, user_id).await {
        Ok(Some(schedule)) => {
            Json(json!({
                "success": true,
                "is_synced": schedule.is_synced,
                "schedule": schedule.get_schedule_dict(),
                "last_synced": schedule.last_synced.map(|dt| dt.format("%Y-%m-%d %H:%M").to_string()),
                "last_modified": schedule.last_modified.format("%Y-%m-%d %H:%M").to_string()
            }))
        }
        Ok(None) => {
            Json(json!({
                "success": true,
                "is_synced": true,
                "schedule": null,
                "last_synced": null,
                "last_modified": null
            }))
        }
        Err(e) => {
            tracing::error!("Database error: {}", e);
            Json(json!({"success": false, "message": "Database error"}))
        }
    }
}