use actix_web::{web, HttpResponse, Result};
use utoipa;

use crate::models::{DashboardResponse, AdminResponse, ServiceError};
use crate::auth::JwtManager;
use crate::middleware::auth::authenticate_request;
use crate::services::UserService;

#[utoipa::path(
    get,
    path = "/api/dashboard",
    responses(
        (status = 200, description = "Dashboard data retrieved", body = DashboardResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse)
    )
)]
pub async fn dashboard_api(
    user_service: web::Data<UserService>,
    req: actix_web::HttpRequest,
    jwt_manager: web::Data<JwtManager>,
) -> Result<HttpResponse, ServiceError> {
    // Authentication
    if let Err(_) = authenticate_request(&req, &jwt_manager) {
        return Err(ServiceError::AuthenticationError(
            "Not authenticated - valid JWT token required".to_string(),
        ));
    }
    
    // Business logic delegation
    let users = user_service.get_dashboard_users().await?;

    Ok(HttpResponse::Ok().json(DashboardResponse {
        success: true,
        users,
    }))
}

#[utoipa::path(
    get,
    path = "/api/admin",
    responses(
        (status = 200, description = "Admin user data retrieved", body = AdminResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse)
    )
)]
pub async fn admin_api(
    user_service: web::Data<UserService>,
    req: actix_web::HttpRequest,
    jwt_manager: web::Data<JwtManager>,
) -> Result<HttpResponse, ServiceError> {
    // Authentication
    if let Err(_) = authenticate_request(&req, &jwt_manager) {
        return Err(ServiceError::AuthenticationError(
            "Not authenticated - valid JWT token required".to_string(),
        ));
    }

    // Business logic delegation
    let users = user_service.get_admin_users().await?;

    Ok(HttpResponse::Ok().json(AdminResponse {
        success: true,
        users,
    }))
}