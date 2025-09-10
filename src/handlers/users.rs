use actix_web::{web, HttpResponse, Result};
use serde_json;
use utoipa;

use crate::models::{AddUserForm, ServiceError};
use crate::auth::JwtManager;
use crate::middleware::auth::authenticate_request;
use crate::services::UserService;

#[utoipa::path(
    post,
    path = "/api/users/add",
    request_body = AddUserForm,
    responses(
        (status = 200, description = "User added successfully", body = ApiResponse),
        (status = 400, description = "Invalid input", body = ErrorResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 409, description = "User already exists", body = ErrorResponse)
    )
)]
pub async fn add_user_api(
    user_service: web::Data<UserService>,
    form: web::Json<AddUserForm>,
    req: actix_web::HttpRequest, 
    jwt_manager: web::Data<JwtManager>,
) -> Result<HttpResponse, ServiceError> {
    // Authentication
    if let Err(_) = authenticate_request(&req, &jwt_manager) {
        return Err(ServiceError::AuthenticationError("Not authenticated".to_string()));
    }

    if form.username.is_empty() || form.system_ip.is_empty() {
        return Err(ServiceError::ValidationError(
            "Both username and system IP are required".to_string()
        ));
    }

    // Business logic delegation
    let message = user_service.add_user(form.username.clone(), form.system_ip.clone()).await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "message": message
    })))
}

#[utoipa::path(
    get,
    path = "/api/users/validate/{id}",
    params(
        ("id" = i64, Path, description = "User ID")
    ),
    responses(
        (status = 200, description = "User validation completed", body = ApiResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse)
    )
)]
pub async fn validate_user(
    user_service: web::Data<UserService>,
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
    let message = user_service.validate_user(user_id).await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "message": message
    })))
}

#[utoipa::path(
    post,
    path = "/api/users/delete/{id}",
    params(
        ("id" = i64, Path, description = "User ID")
    ),
    responses(
        (status = 200, description = "User deleted successfully", body = ApiResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse),
        (status = 500, description = "Failed to delete user", body = ErrorResponse)
    )
)]
pub async fn delete_user(
    user_service: web::Data<UserService>,
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
    let message = user_service.delete_user(user_id).await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "message": message
    })))
}