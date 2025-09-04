use axum::{
    extract::{Path, State},
    response::{Html, Redirect, Response, IntoResponse},
    Form,
};
use serde::Deserialize;
use tower_sessions::Session;
use std::collections::HashMap;

use crate::{
    database::models::{ManagedUser, UserWeeklySchedule},
    utils::auth::is_authenticated,
    AppState,
};

pub async fn dashboard(State(state): State<AppState>, session: Session) -> Response {
    if !is_authenticated(&session).await {
        return Redirect::to("/").into_response();
    }
    
    match ManagedUser::get_valid_users(&state.pool).await {
        Ok(users) => {
            let mut user_data = Vec::new();
            let mut pending_adjustments = HashMap::new();
            
            for user in users {
                // Get usage data
                let usage_data = user.get_recent_usage(&state.pool, 7).await.unwrap_or_default();
                
                // Get time left
                let time_left = user.get_config_value("TIME_LEFT_DAY")
                    .and_then(|v| v.as_i64())
                    .map(|seconds| {
                        let hours = seconds / 3600;
                        let minutes = (seconds % 3600) / 60;
                        format!("{}h {}m", hours, minutes)
                    })
                    .unwrap_or_else(|| "Unknown".to_string());
                
                // Check for pending adjustments
                if let (Some(adjustment), Some(operation)) = (&user.pending_time_adjustment, &user.pending_time_operation) {
                    let minutes = adjustment / 60;
                    pending_adjustments.insert(user.id.to_string(), format!("{}{} minutes", operation, minutes));
                }
                
                user_data.push(serde_json::json!({
                    "id": user.id,
                    "username": user.username,
                    "system_ip": user.system_ip,
                    "last_checked": user.last_checked,
                    "usage_data": usage_data,
                    "time_left": time_left,
                }));
            }
            
            // Simple template replacement for now
            let html = include_str!("../../templates/dashboard.html")
                .replace("{{ users }}", &serde_json::to_string(&user_data).unwrap_or_default())
                .replace("{{ pending_adjustments }}", &serde_json::to_string(&pending_adjustments).unwrap_or_default());
            
            Html(html).into_response()
        }
        Err(e) => {
            tracing::error!("Database error: {}", e);
            Html("Database error").into_response()
        }
    }
}

#[derive(Deserialize)]
pub struct WeeklyScheduleForm {
    user_id: i64,
    monday: Option<f64>,
    tuesday: Option<f64>,
    wednesday: Option<f64>,
    thursday: Option<f64>,
    friday: Option<f64>,
    saturday: Option<f64>,
    sunday: Option<f64>,
}

pub async fn weekly_schedule_user(
    State(state): State<AppState>,
    session: Session,
    Path(user_id): Path<i64>,
) -> Response {
    if !is_authenticated(&session).await {
        return Redirect::to("/").into_response();
    }
    
    // Get user and ensure they have a weekly schedule
    match sqlx::query_as::<_, ManagedUser>(
        "SELECT id, username, system_ip, is_valid, date_added, last_checked, last_config, pending_time_adjustment, pending_time_operation FROM managed_user WHERE id = ?"
    )
    .bind(user_id)
    .fetch_optional(&state.pool)
    .await {
        Ok(Some(user)) => {
            // Ensure weekly schedule exists
            if UserWeeklySchedule::get_by_user_id(&state.pool, user_id).await.unwrap_or(None).is_none() {
                let empty_schedule = HashMap::new();
                let _ = UserWeeklySchedule::create_or_update(&state.pool, user_id, &empty_schedule).await;
            }
            
            let html = include_str!("../../templates/weekly_schedule_single.html")
                .replace("{{ user.id }}", &user.id.to_string())
                .replace("{{ user.username }}", &user.username);
            
            Html(html).into_response()
        }
        Ok(None) => Html("User not found").into_response(),
        Err(e) => {
            tracing::error!("Database error: {}", e);
            Html("Database error").into_response()
        }
    }
}

pub async fn update_weekly_schedule(
    State(state): State<AppState>,
    session: Session,
    Form(form): Form<WeeklyScheduleForm>,
) -> Response {
    if !is_authenticated(&session).await {
        return Redirect::to("/").into_response();
    }
    
    let mut schedule = HashMap::new();
    schedule.insert("monday".to_string(), form.monday.unwrap_or(0.0).max(0.0).min(24.0));
    schedule.insert("tuesday".to_string(), form.tuesday.unwrap_or(0.0).max(0.0).min(24.0));
    schedule.insert("wednesday".to_string(), form.wednesday.unwrap_or(0.0).max(0.0).min(24.0));
    schedule.insert("thursday".to_string(), form.thursday.unwrap_or(0.0).max(0.0).min(24.0));
    schedule.insert("friday".to_string(), form.friday.unwrap_or(0.0).max(0.0).min(24.0));
    schedule.insert("saturday".to_string(), form.saturday.unwrap_or(0.0).max(0.0).min(24.0));
    schedule.insert("sunday".to_string(), form.sunday.unwrap_or(0.0).max(0.0).min(24.0));
    
    match UserWeeklySchedule::create_or_update(&state.pool, form.user_id, &schedule).await {
        Ok(()) => Redirect::to(&format!("/weekly-schedule/{}", form.user_id)).into_response(),
        Err(e) => {
            tracing::error!("Error updating weekly schedule: {}", e);
            Html("Error updating schedule").into_response()
        }
    }
}