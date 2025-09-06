use actix_web::{web, HttpResponse, Result};
use sqlx::SqlitePool;
use serde_json;
use utoipa;

use crate::models::{LoginForm, PasswordChangeForm, LoginResponse, ApiResponse, ServiceError};
use crate::auth::JwtManager;
use crate::middleware::auth::authenticate_request;

#[utoipa::path(
    post,
    path = "/api/login",
    request_body = LoginForm,
    responses(
        (status = 200, description = "Login successful - JWT token returned in response body", body = LoginResponse),
        (status = 401, description = "Invalid credentials", body = ErrorResponse)
    ),
    security()
)]
pub async fn login_api(
    pool: web::Data<SqlitePool>,
    form: web::Json<LoginForm>,
    jwt_manager: web::Data<JwtManager>,
) -> Result<HttpResponse, ServiceError> {
    if form.username == "admin" {
        // Check admin password
        let admin_hash = sqlx::query_scalar::<_, String>(
            "SELECT value FROM settings WHERE key = 'admin_password_hash'"
        )
        .fetch_optional(pool.get_ref())
        .await;

        match admin_hash {
            Ok(Some(hash)) => {
                use argon2::{Argon2, PasswordVerifier, PasswordHash};
                
                if let Ok(parsed_hash) = PasswordHash::new(&hash) {
                    if Argon2::default().verify_password(form.password.as_bytes(), &parsed_hash).is_ok() {
                        // Generate JWT token
                        match jwt_manager.generate_token(&form.username) {
                            Ok(token) => {
                                return Ok(HttpResponse::Ok().json(LoginResponse {
                                    success: true,
                                    message: "Login successful".to_string(),
                                    token,
                                    expires_in: 24 * 3600, // 24 hours in seconds
                                }));
                            }
                            Err(_) => {
                                return Err(ServiceError::InternalError("Failed to generate token".to_string()));
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
    
    // Login failed
    Err(ServiceError::AuthenticationError("Invalid credentials".to_string()))
}

#[utoipa::path(
    post,
    path = "/api/logout",
    responses(
        (status = 200, description = "Logout successful", body = ApiResponse)
    ),
    security()
)]
pub async fn logout_api() -> Result<HttpResponse, ServiceError> {
    // With JWT, logout is handled client-side by discarding the token
    // Server doesn't need to track token state
    Ok(HttpResponse::Ok().json(ApiResponse {
        success: true,
        message: "Logout successful - discard your token".to_string(),
    }))
}

#[utoipa::path(
    post,
    path = "/api/change-password",
    request_body = PasswordChangeForm,
    responses(
        (status = 200, description = "Password changed successfully", body = ApiResponse),
        (status = 400, description = "Invalid input", body = ErrorResponse),
        (status = 401, description = "Authentication failed", body = ErrorResponse)
    )
)]
pub async fn change_password_api(
    pool: web::Data<SqlitePool>,
    form: web::Json<PasswordChangeForm>,
    req: actix_web::HttpRequest, 
    jwt_manager: web::Data<JwtManager>,
) -> Result<HttpResponse, ServiceError> {
    if let Err(_) = authenticate_request(&req, &jwt_manager) {
        return Err(ServiceError::AuthenticationError("Not authenticated".to_string()));
    }

    // Validate inputs
    if form.current_password.is_empty() || form.new_password.is_empty() || form.confirm_password.is_empty() {
        return Err(ServiceError::ValidationError("All fields are required".to_string()));
    }

    if form.new_password != form.confirm_password {
        return Err(ServiceError::ValidationError("New passwords do not match".to_string()));
    }

    if form.new_password.len() < 4 {
        return Err(ServiceError::ValidationError("New password must be at least 4 characters long".to_string()));
    }

    // Check current password
    let admin_hash = sqlx::query_scalar::<_, String>(
        "SELECT value FROM settings WHERE key = 'admin_password_hash'"
    )
    .fetch_optional(pool.get_ref())
    .await;

    match admin_hash {
        Ok(Some(hash)) => {
            use argon2::{Argon2, PasswordVerifier, PasswordHash};
            
            if let Ok(parsed_hash) = PasswordHash::new(&hash) {
                if Argon2::default().verify_password(form.current_password.as_bytes(), &parsed_hash).is_ok() {
                    // Current password is correct, update to new password
                    use argon2::PasswordHasher;
                    use argon2::password_hash::{rand_core::OsRng, SaltString};
                    
                    let salt = SaltString::generate(&mut OsRng);
                    let argon2 = Argon2::default();
                    let new_password_hash = argon2.hash_password(form.new_password.as_bytes(), &salt);
                    
                    match new_password_hash {
                        Ok(hash) => {
                            let result = sqlx::query(
                                "INSERT OR REPLACE INTO settings (key, value) VALUES ('admin_password_hash', ?)"
                            )
                            .bind(hash.to_string())
                            .execute(pool.get_ref())
                            .await;

                            match result {
                                Ok(_) => {
                                    println!("Admin password updated successfully");
                                    Ok(HttpResponse::Ok().json(serde_json::json!({
                                        "success": true,
                                        "message": "Password updated successfully"
                                    })))
                                }
                                Err(e) => {
                                    eprintln!("Failed to update password: {}", e);
                                    Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                                        "success": false,
                                        "message": "Failed to update password"
                                    })))
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("Failed to hash new password: {}", e);
                            Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                                "success": false,
                                "message": "Failed to process new password"
                            })))
                        }
                    }
                } else {
                    Ok(HttpResponse::Unauthorized().json(serde_json::json!({
                        "success": false,
                        "message": "Current password is incorrect"
                    })))
                }
            } else {
                Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                    "success": false,
                    "message": "System error. Please try again."
                })))
            }
        }
        _ => {
            Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "success": false,
                "message": "System error. Please try again."
            })))
        }
    }
}