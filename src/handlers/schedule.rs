use actix_web::{web, HttpResponse, Result};
use serde_json;
use utoipa;

use crate::models::{ScheduleUpdateForm, WeeklyHours, ServiceError};
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

    // Business logic delegation - service handles all business rules
    schedule_service.update_schedule(form.user_id, hours).await?;

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
        (status = 200, description = "Schedule sync status retrieved"),
        (status = 401, description = "Not authenticated")
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