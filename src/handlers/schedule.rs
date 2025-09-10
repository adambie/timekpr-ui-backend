use actix_web::{web, HttpResponse, Result};
use serde_json;
use utoipa;

use crate::models::{ScheduleUpdateForm, WeeklyHours, WeeklyTimeIntervals, TimeInterval, ServiceError};
use crate::auth::JwtManager;
use crate::middleware::auth::authenticate_request;
use crate::services::ScheduleService;

#[utoipa::path(
    post,
    path = "/api/schedule/update",
    request_body = ScheduleUpdateForm,
    responses(
        (status = 200, description = "Schedule updated successfully"),
        (status = 400, description = "Invalid schedule values"),
        (status = 401, description = "Not authenticated")
    )
)]
pub async fn update_schedule_api(
    schedule_service: web::Data<ScheduleService>,
    form: web::Json<ScheduleUpdateForm>,
    req: actix_web::HttpRequest, 
    jwt_manager: web::Data<JwtManager>,
) -> Result<HttpResponse, ServiceError> {
    // Authentication - only HTTP concern
    if let Err(_) = authenticate_request(&req, &jwt_manager) {
        return Err(ServiceError::AuthenticationError("Not authenticated".to_string()));
    }

    println!("Received schedule update: user_id={}, monday={}, tuesday={}, wednesday={}, thursday={}, friday={}, saturday={}, sunday={}",
             form.user_id, form.monday, form.tuesday, form.wednesday, form.thursday, form.friday, form.saturday, form.sunday);

    // Convert API model to domain model
    let hours = WeeklyHours {
        monday: form.monday,
        tuesday: form.tuesday,
        wednesday: form.wednesday,
        thursday: form.thursday,
        friday: form.friday,
        saturday: form.saturday,
        sunday: form.sunday,
    };

    // Check if time intervals are provided
    let has_intervals = form.monday_start_time.is_some() || form.tuesday_start_time.is_some() ||
                       form.wednesday_start_time.is_some() || form.thursday_start_time.is_some() ||
                       form.friday_start_time.is_some() || form.saturday_start_time.is_some() ||
                       form.sunday_start_time.is_some();

    if has_intervals {
        // Build time intervals using TimeInterval::new for validation
        let monday_interval = TimeInterval::new(
            form.monday_start_time.clone().unwrap_or("00:00".to_string()),
            form.monday_end_time.clone().unwrap_or("23:59".to_string())
        ).map_err(|e| ServiceError::ValidationError(format!("Monday interval: {}", e)))?;
        
        let tuesday_interval = TimeInterval::new(
            form.tuesday_start_time.clone().unwrap_or("00:00".to_string()),
            form.tuesday_end_time.clone().unwrap_or("23:59".to_string())
        ).map_err(|e| ServiceError::ValidationError(format!("Tuesday interval: {}", e)))?;
        
        let wednesday_interval = TimeInterval::new(
            form.wednesday_start_time.clone().unwrap_or("00:00".to_string()),
            form.wednesday_end_time.clone().unwrap_or("23:59".to_string())
        ).map_err(|e| ServiceError::ValidationError(format!("Wednesday interval: {}", e)))?;
        
        let thursday_interval = TimeInterval::new(
            form.thursday_start_time.clone().unwrap_or("00:00".to_string()),
            form.thursday_end_time.clone().unwrap_or("23:59".to_string())
        ).map_err(|e| ServiceError::ValidationError(format!("Thursday interval: {}", e)))?;
        
        let friday_interval = TimeInterval::new(
            form.friday_start_time.clone().unwrap_or("00:00".to_string()),
            form.friday_end_time.clone().unwrap_or("23:59".to_string())
        ).map_err(|e| ServiceError::ValidationError(format!("Friday interval: {}", e)))?;
        
        let saturday_interval = TimeInterval::new(
            form.saturday_start_time.clone().unwrap_or("00:00".to_string()),
            form.saturday_end_time.clone().unwrap_or("23:59".to_string())
        ).map_err(|e| ServiceError::ValidationError(format!("Saturday interval: {}", e)))?;
        
        let sunday_interval = TimeInterval::new(
            form.sunday_start_time.clone().unwrap_or("00:00".to_string()),
            form.sunday_end_time.clone().unwrap_or("23:59".to_string())
        ).map_err(|e| ServiceError::ValidationError(format!("Sunday interval: {}", e)))?;
        
        let intervals = WeeklyTimeIntervals {
            monday: monday_interval,
            tuesday: tuesday_interval,
            wednesday: wednesday_interval,
            thursday: thursday_interval,
            friday: friday_interval,
            saturday: saturday_interval,
            sunday: sunday_interval,
        };

        // Business logic delegation - service handles all business rules with intervals
        schedule_service.update_schedule_with_intervals(form.user_id, hours, intervals).await?;
    } else {
        // Business logic delegation - service handles all business rules (backward compatibility)
        schedule_service.update_schedule(form.user_id, hours).await?;
    }

    // Success response
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "message": "Schedule updated successfully"
    })))
}

#[utoipa::path(
    get,
    path = "/api/schedule-sync-status/{id}",
    params(
        ("id" = i64, Path, description = "User ID")
    ),
    responses(
        (status = 200, description = "Schedule sync status retrieved", body = ScheduleSyncResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse)
    )
)]
pub async fn get_schedule_sync_status(
    schedule_service: web::Data<ScheduleService>,
    path: web::Path<i64>,
    req: actix_web::HttpRequest, 
    jwt_manager: web::Data<JwtManager>,
) -> Result<HttpResponse, ServiceError> {
    // Authentication
    if let Err(_) = authenticate_request(&req, &jwt_manager) {
        return Err(ServiceError::AuthenticationError("Not authenticated".to_string()));
    }

    let user_id = path.into_inner();
    
    // Business logic delegation
    let sync_status = schedule_service.get_sync_status(user_id).await?;

    println!("Retrieved schedule sync status for user {}", user_id);
    
    // Response formatting
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "is_synced": sync_status.is_synced,
        "schedule": sync_status.schedule,
        "last_synced": sync_status.last_synced,
        "last_modified": sync_status.last_modified
    })))
}