use actix_web::{web, HttpResponse, Result};
use serde_json;
use sqlx::SqlitePool;
use utoipa;

use crate::auth::JwtManager;
use crate::middleware::auth::authenticate_request;
use crate::models::{ServiceError, SshStatusResponse};
use crate::scheduler::BackgroundScheduler;
use crate::ssh::SSHClient;

#[utoipa::path(
    get,
    path = "/api/task-status",
    responses(
        (status = 200, description = "Background task status retrieved", body = TaskStatusResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse)
    )
)]
pub async fn get_task_status(
    pool: web::Data<SqlitePool>,
    req: actix_web::HttpRequest,
    jwt_manager: web::Data<JwtManager>,
    scheduler: web::Data<std::sync::Arc<BackgroundScheduler>>,
) -> Result<HttpResponse, ServiceError> {
    // Authentication
    if let Err(_) = authenticate_request(&req, &jwt_manager) {
        return Err(ServiceError::AuthenticationError(
            "Not authenticated".to_string(),
        ));
    }

    // Get actual status
    let is_running = scheduler.is_running().await;

    // Count managed users
    let user_count =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM managed_users WHERE is_valid = 1")
            .fetch_one(pool.get_ref())
            .await
            .unwrap_or(0);

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "status": {
            "running": is_running,
            "last_update": chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            "managed_users": user_count
        }
    })))
}

#[utoipa::path(
    get,
    path = "/api/ssh-status",
    responses(
        (status = 200, description = "SSH status retrieved", body = SshStatusResponse),
        (status = 401, description = "Not authenticated", body = ErrorResponse)
    )
)]
pub async fn get_ssh_status(
    req: actix_web::HttpRequest,
    jwt_manager: web::Data<JwtManager>,
) -> Result<HttpResponse, ServiceError> {
    // Authentication
    if let Err(_) = authenticate_request(&req, &jwt_manager) {
        return Err(ServiceError::AuthenticationError(
            "Not authenticated".to_string(),
        ));
    }

    let ssh_key_exists = SSHClient::check_ssh_key_exists();

    Ok(HttpResponse::Ok().json(SshStatusResponse {
        success: true,
        ssh_key_exists,
        message: if ssh_key_exists {
            "SSH keys are configured".to_string()
        } else {
            "SSH keys not found. Please configure SSH keys for passwordless authentication."
                .to_string()
        },
    }))
}
