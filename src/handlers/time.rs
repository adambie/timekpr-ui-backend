use actix_web::{web, HttpResponse, Result};
use serde_json;
use utoipa;

use crate::auth::JwtManager;
use crate::middleware::auth::authenticate_request;
use crate::models::{ModifyTimeForm, ServiceError, TimeModification};
use crate::services::TimeService;

#[utoipa::path(
    post,
    path = "/api/modify-time",
    request_body = ModifyTimeForm,
    responses(
        (status = 200, description = "Time modified successfully", body = ModifyTimeResponse),
        (status = 400, description = "Invalid operation", body = ErrorResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 404, description = "User not found", body = ErrorResponse)
    )
)]
pub async fn modify_time(
    time_service: web::Data<TimeService>,
    form: web::Json<ModifyTimeForm>,
    req: actix_web::HttpRequest,
    jwt_manager: web::Data<JwtManager>,
) -> Result<HttpResponse, ServiceError> {
    // Authentication
    if let Err(_) = authenticate_request(&req, &jwt_manager) {
        return Err(ServiceError::AuthenticationError(
            "Not authenticated".to_string(),
        ));
    }

    // Create domain object with validation
    let modification = TimeModification::new(form.user_id, form.operation.clone(), form.seconds)
        .map_err(|e| ServiceError::ValidationError(e))?;

    // Business logic delegation
    let result = time_service.modify_time(modification).await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": result.success,
        "message": result.message,
        "username": result.username,
        "pending": result.pending,
        "refresh": true
    })))
}

#[utoipa::path(
    get,
    path = "/api/user/{id}/usage",
    params(
        ("id" = i64, Path, description = "User ID")
    ),
    responses(
        (status = 200, description = "User usage data retrieved", body = UsageResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 404, description = "User not found", body = ErrorResponse)
    )
)]
pub async fn get_user_usage(
    time_service: web::Data<TimeService>,
    path: web::Path<i64>,
    req: actix_web::HttpRequest,
    jwt_manager: web::Data<JwtManager>,
) -> Result<HttpResponse, ServiceError> {
    // Authentication
    if let Err(_) = authenticate_request(&req, &jwt_manager) {
        return Err(ServiceError::AuthenticationError(
            "Not authenticated".to_string(),
        ));
    }

    let user_id = path.into_inner();

    // Business logic delegation
    let usage_data = time_service.get_user_usage(user_id).await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "data": usage_data.usage_data,
        "username": usage_data.username
    })))
}
