use actix_web::{HttpResponse, ResponseError};
use serde_json::json;
use std::fmt;
use std::error::Error as StdError;

#[derive(Debug)]
pub enum ServiceError {
    ValidationError(String),
    DatabaseError(String),
    #[allow(dead_code)]
    SshError(String),
    NotFound(String),
    AuthenticationError(String),
    InternalError(String),
}

impl fmt::Display for ServiceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ServiceError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
            ServiceError::DatabaseError(msg) => write!(f, "Database error: {}", msg),
            ServiceError::SshError(msg) => write!(f, "SSH error: {}", msg),
            ServiceError::NotFound(msg) => write!(f, "Not found: {}", msg),
            ServiceError::AuthenticationError(msg) => write!(f, "Authentication error: {}", msg),
            ServiceError::InternalError(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl StdError for ServiceError {}

impl ResponseError for ServiceError {
    fn error_response(&self) -> HttpResponse {
        match self {
            ServiceError::ValidationError(msg) => HttpResponse::BadRequest().json(json!({
                "success": false,
                "message": msg
            })),
            ServiceError::NotFound(msg) => HttpResponse::NotFound().json(json!({
                "success": false,
                "message": msg
            })),
            ServiceError::AuthenticationError(msg) => HttpResponse::Unauthorized().json(json!({
                "success": false,
                "message": msg
            })),
            ServiceError::DatabaseError(msg) => {
                eprintln!("Database error: {}", msg);
                HttpResponse::InternalServerError().json(json!({
                    "success": false,
                    "message": "Database error occurred"
                }))
            }
            ServiceError::SshError(msg) => HttpResponse::Ok().json(json!({
                "success": true,
                "message": format!("Queued for later sync: {}", msg),
                "pending": true
            })),
            ServiceError::InternalError(msg) => {
                eprintln!("Internal error: {}", msg);
                HttpResponse::InternalServerError().json(json!({
                    "success": false,
                    "message": "Internal server error"
                }))
            }
        }
    }
}

// Conversion from sqlx errors
impl From<sqlx::Error> for ServiceError {
    fn from(err: sqlx::Error) -> Self {
        ServiceError::DatabaseError(err.to_string())
    }
}
