use actix_web::{web, App, HttpServer, HttpResponse, Result, middleware};
use actix_session::{Session, SessionMiddleware, storage::CookieSessionStore};
use actix_web::cookie::Key;
use actix_cors::Cors;
use serde::{Deserialize, Serialize};
use serde_json;
use sqlx::{SqlitePool, FromRow};
use chrono::{DateTime, Utc};

mod ssh;
mod scheduler;
use ssh::SSHClient;
use scheduler::BackgroundScheduler;

#[derive(Deserialize)]
struct ScheduleUpdateForm {
    user_id: i64,
    monday: f64,
    tuesday: f64,
    wednesday: f64,
    thursday: f64,
    friday: f64,
    saturday: f64,
    sunday: f64,
}

#[derive(Deserialize)]
struct LoginForm {
    username: String,
    password: String,
}

#[derive(Deserialize)]
struct AddUserForm {
    username: String,
    system_ip: String,
}

#[derive(Deserialize)]
struct ModifyTimeForm {
    user_id: i64,
    operation: String,
    seconds: i64,
}

#[derive(Deserialize)]
struct PasswordChangeForm {
    current_password: String,
    new_password: String,
    confirm_password: String,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
struct ManagedUser {
    pub id: i64,
    pub username: String,
    pub system_ip: String,
    pub is_valid: bool,
    pub last_checked: Option<DateTime<Utc>>,
    pub last_config: Option<String>,
    pub pending_time_adjustment: Option<i64>,
    pub pending_time_operation: Option<String>,
}

// Remove - frontend will serve its own login page

async fn login_api(
    pool: web::Data<SqlitePool>,
    form: web::Json<LoginForm>,
    session: Session,
) -> Result<HttpResponse> {
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
                        session.insert("logged_in", true).map_err(|_| actix_web::error::ErrorInternalServerError("Session error"))?;
                        return Ok(HttpResponse::Ok().json(serde_json::json!({
                            "success": true,
                            "message": "Login successful"
                        })));
                    }
                }
            }
            _ => {}
        }
    }
    
    // Login failed
    Ok(HttpResponse::Unauthorized().json(serde_json::json!({
        "success": false,
        "message": "Invalid credentials"
    })))
}

async fn logout_api(session: Session) -> Result<HttpResponse> {
    session.remove("logged_in");
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "message": "Logged out successfully"
    })))
}

fn is_authenticated(session: &Session) -> bool {
    session.get::<bool>("logged_in").unwrap_or(Some(false)).unwrap_or(false)
}

async fn dashboard_api(
    pool: web::Data<SqlitePool>, 
    session: Session
) -> Result<HttpResponse> {
    if !is_authenticated(&session) {
        return Ok(HttpResponse::Unauthorized().json(serde_json::json!({
            "success": false,
            "message": "Not authenticated"
        })));
    }
    
    // Get all valid users from database
    let users = sqlx::query_as::<_, ManagedUser>(
        "SELECT * FROM managed_users WHERE is_valid = 1 ORDER BY username"
    )
    .fetch_all(pool.get_ref())
    .await
    .unwrap_or_else(|e| {
        eprintln!("Failed to fetch users: {}", e);
        vec![]
    });

    // Create user data for API response
    let mut user_data = Vec::new();
    for user in users {
        let time_left_formatted = if let Some(config_str) = &user.last_config {
            // Parse the JSON config to get actual time left
            if let Ok(config) = serde_json::from_str::<serde_json::Value>(config_str) {
                if let Some(time_left) = config.get("TIME_LEFT_DAY").and_then(|v| v.as_i64()) {
                    let hours = time_left / 3600;
                    let minutes = (time_left % 3600) / 60;
                    format!("{}h {}m", hours, minutes)
                } else {
                    "No limit set".to_string()
                }
            } else {
                "Unknown".to_string()
            }
        } else {
            "Unknown".to_string()
        };

        let last_checked_str = user.last_checked
            .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
            .unwrap_or_else(|| "Never".to_string());

        let pending_adjustment = if let (Some(adjustment), Some(operation)) = 
            (&user.pending_time_adjustment, &user.pending_time_operation) {
            Some(format!("{}{} minutes", operation, adjustment / 60))
        } else {
            None
        };

        // Check for unsynced schedule changes
        let pending_schedule = sqlx::query_scalar::<_, bool>(
            "SELECT COUNT(*) > 0 FROM user_weekly_schedule WHERE user_id = ? AND is_synced = 0"
        )
        .bind(user.id)
        .fetch_one(pool.get_ref())
        .await
        .unwrap_or(false);

        println!("User {}: time_left_formatted = '{}', config = {:?}", user.username, time_left_formatted, user.last_config);
        
        user_data.push(serde_json::json!({
            "id": user.id,
            "username": user.username,
            "system_ip": user.system_ip,
            "time_left": time_left_formatted,
            "last_checked": last_checked_str,
            "pending_adjustment": pending_adjustment,
            "pending_schedule": pending_schedule
        }));
    }

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "users": user_data
    })))
}

async fn admin_api(
    pool: web::Data<SqlitePool>, 
    session: Session
) -> Result<HttpResponse> {
    if !is_authenticated(&session) {
        return Ok(HttpResponse::Unauthorized().json(serde_json::json!({
            "success": false,
            "message": "Not authenticated"
        })));
    }

    // Get all users from database
    let users = sqlx::query_as::<_, ManagedUser>(
        "SELECT * FROM managed_users ORDER BY username"
    )
    .fetch_all(pool.get_ref())
    .await
    .unwrap_or_else(|e| {
        eprintln!("Failed to fetch users: {}", e);
        vec![]
    });

    // Create user data for API response
    let mut user_data = Vec::new();
    for user in users {
        let last_checked_str = user.last_checked
            .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
            .unwrap_or_else(|| "Never".to_string());

        user_data.push(serde_json::json!({
            "id": user.id,
            "username": user.username,
            "system_ip": user.system_ip,
            "is_valid": user.is_valid,
            "last_checked": last_checked_str
        }));
    }

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "users": user_data
    })))
}

// Remove - settings will be handled by password change API endpoint

async fn change_password_api(
    pool: web::Data<SqlitePool>,
    form: web::Json<PasswordChangeForm>,
    session: Session,
) -> Result<HttpResponse> {
    if !is_authenticated(&session) {
        return Ok(HttpResponse::Unauthorized().json(serde_json::json!({
            "success": false,
            "message": "Not authenticated"
        })));
    }

    // Validate inputs
    if form.current_password.is_empty() || form.new_password.is_empty() || form.confirm_password.is_empty() {
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "success": false,
            "message": "All fields are required"
        })));
    }

    if form.new_password != form.confirm_password {
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "success": false,
            "message": "New passwords do not match"
        })));
    }

    if form.new_password.len() < 4 {
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "success": false,
            "message": "New password must be at least 4 characters long"
        })));
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

async fn add_user_api(
    pool: web::Data<SqlitePool>,
    form: web::Json<AddUserForm>,
    session: Session,
) -> Result<HttpResponse> {
    if !is_authenticated(&session) {
        return Ok(HttpResponse::Unauthorized().json("Not authenticated"));
    }

    if form.username.is_empty() || form.system_ip.is_empty() {
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "success": false,
            "message": "Both username and system IP are required"
        })));
    }

    // Check if user already exists
    let existing = sqlx::query_as::<_, ManagedUser>(
        "SELECT * FROM managed_users WHERE username = ? AND system_ip = ?"
    )
    .bind(&form.username)
    .bind(&form.system_ip)
    .fetch_optional(pool.get_ref())
    .await;

    if let Ok(Some(_)) = existing {
        return Ok(HttpResponse::Conflict().json(serde_json::json!({
            "success": false,
            "message": format!("User {} on {} already exists", form.username, form.system_ip)
        })));
    }

    // Validate user with SSH and timekpr
    let ssh_client = SSHClient::new(&form.system_ip);
    let (is_valid, message, config) = ssh_client.validate_user(&form.username).await;
    
    let config_json = config.map(|c| c.to_string());

    // Create new user with validation results
    let result = sqlx::query(
        "INSERT INTO managed_users (username, system_ip, is_valid, last_checked, last_config) VALUES (?, ?, ?, ?, ?)"
    )
    .bind(&form.username)
    .bind(&form.system_ip)
    .bind(is_valid)
    .bind(Utc::now())
    .bind(config_json)
    .execute(pool.get_ref())
    .await;

    match result {
        Ok(_) => {
            if is_valid {
                println!("Added and validated user: {} on {} - {}", form.username, form.system_ip, message);
                Ok(HttpResponse::Ok().json(serde_json::json!({
                    "success": true,
                    "message": format!("User {} added and validated successfully", form.username)
                })))
            } else {
                println!("Added user: {} on {} but validation failed: {}", form.username, form.system_ip, message);
                Ok(HttpResponse::Ok().json(serde_json::json!({
                    "success": true,
                    "message": format!("User {} added but validation failed: {}", form.username, message)
                })))
            }
        }
        Err(e) => {
            eprintln!("Failed to add user: {}", e);
            Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "success": false,
                "message": "Failed to add user to database"
            })))
        }
    }
}

async fn validate_user(
    pool: web::Data<SqlitePool>,
    path: web::Path<i64>,
    session: Session,
) -> Result<HttpResponse> {
    if !is_authenticated(&session) {
        return Ok(HttpResponse::Unauthorized().json("Not authenticated"));
    }

    let user_id = path.into_inner();
    
    // Get user from database
    let user = sqlx::query_as::<_, ManagedUser>(
        "SELECT * FROM managed_users WHERE id = ?"
    )
    .bind(user_id)
    .fetch_optional(pool.get_ref())
    .await;

    if let Ok(Some(user)) = user {
        // Validate with SSH and timekpr
        let ssh_client = SSHClient::new(&user.system_ip);
        let (is_valid, message, config) = ssh_client.validate_user(&user.username).await;
        
        let config_json = config.map(|c| c.to_string());
        
        let _result = sqlx::query(
            "UPDATE managed_users SET last_checked = ?, is_valid = ?, last_config = ? WHERE id = ?"
        )
        .bind(Utc::now())
        .bind(is_valid)
        .bind(config_json)
        .bind(user_id)
        .execute(pool.get_ref())
        .await;

        if is_valid {
            println!("Validated user: {} - {}", user.username, message);
        } else {
            println!("Validation failed for user: {} - {}", user.username, message);
        }
    }

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "message": "User validation completed"
    })))
}

async fn delete_user(
    pool: web::Data<SqlitePool>,
    path: web::Path<i64>,
    session: Session,
) -> Result<HttpResponse> {
    if !is_authenticated(&session) {
        return Ok(HttpResponse::Unauthorized().json("Not authenticated"));
    }

    let user_id = path.into_inner();

    let result = sqlx::query("DELETE FROM managed_users WHERE id = ?")
        .bind(user_id)
        .execute(pool.get_ref())
        .await;

    match result {
        Ok(_) => {
            println!("Deleted user with id: {}", user_id);
            Ok(HttpResponse::Ok().json(serde_json::json!({
                "success": true,
                "message": "User deleted successfully"
            })))
        }
        Err(e) => {
            eprintln!("Failed to delete user: {}", e);
            Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "success": false,
                "message": "Failed to delete user"
            })))
        }
    }
}

async fn modify_time(
    pool: web::Data<SqlitePool>,
    form: web::Json<ModifyTimeForm>,
    session: Session,
) -> Result<HttpResponse> {
    if !is_authenticated(&session) {
        return Ok(HttpResponse::Unauthorized().json(serde_json::json!({
            "success": false,
            "message": "Not authenticated"
        })));
    }

    // Validate operation
    if form.operation != "+" && form.operation != "-" {
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "success": false,
            "message": "Operation must be '+' or '-'"
        })));
    }

    // Get user from database
    let user = sqlx::query_as::<_, ManagedUser>(
        "SELECT * FROM managed_users WHERE id = ?"
    )
    .bind(form.user_id)
    .fetch_optional(pool.get_ref())
    .await;

    match user {
        Ok(Some(user)) => {
            // Try to apply the time modification via SSH
            let ssh_client = SSHClient::new(&user.system_ip);
            let (success, message) = ssh_client.modify_time_left(&user.username, &form.operation, form.seconds).await;
            
            if success {
                // Command succeeded, update user info and clear pending adjustments
                let ssh_client = SSHClient::new(&user.system_ip);
                let (is_valid, _, config) = ssh_client.validate_user(&user.username).await;
                
                if is_valid {
                    let config_json = config.map(|c| c.to_string());
                    let _result = sqlx::query(
                        "UPDATE managed_users SET last_checked = ?, last_config = ?, pending_time_adjustment = NULL, pending_time_operation = NULL WHERE id = ?"
                    )
                    .bind(Utc::now())
                    .bind(config_json)
                    .bind(form.user_id)
                    .execute(pool.get_ref())
                    .await;
                }
                
                println!("Applied time adjustment: {}{}s for user {} - {}", form.operation, form.seconds, user.username, message);
                Ok(HttpResponse::Ok().json(serde_json::json!({
                    "success": true,
                    "message": message,
                    "username": user.username,
                    "refresh": true
                })))
            } else {
                // Command failed, store as pending adjustment
                let result = sqlx::query(
                    "UPDATE managed_users SET pending_time_adjustment = ?, pending_time_operation = ? WHERE id = ?"
                )
                .bind(form.seconds)
                .bind(&form.operation)
                .bind(form.user_id)
                .execute(pool.get_ref())
                .await;

                match result {
                    Ok(_) => {
                        println!("Queued time adjustment: {}{}s for user {} - SSH failed: {}", form.operation, form.seconds, user.username, message);
                        Ok(HttpResponse::Ok().json(serde_json::json!({
                            "success": true,
                            "message": format!("Computer seems to be offline. Time adjustment of {}{}s has been queued and will be applied when the computer comes online.", form.operation, form.seconds),
                            "username": user.username,
                            "pending": true,
                            "refresh": true
                        })))
                    }
                    Err(e) => {
                        eprintln!("Failed to store pending adjustment: {}", e);
                        Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                            "success": false,
                            "message": "Failed to store time adjustment"
                        })))
                    }
                }
            }
        }
        Ok(None) => Ok(HttpResponse::NotFound().json(serde_json::json!({
            "success": false,
            "message": "User not found"
        }))),
        Err(e) => {
            eprintln!("Database error: {}", e);
            Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "success": false,
                "message": "Database error"
            })))
        }
    }
}

async fn get_user_usage(
    pool: web::Data<SqlitePool>,
    path: web::Path<i64>,
    session: Session,
) -> Result<HttpResponse> {
    if !is_authenticated(&session) {
        return Ok(HttpResponse::Unauthorized().json(serde_json::json!({
            "success": false,
            "message": "Not authenticated"
        })));
    }

    let user_id = path.into_inner();
    
    // Get user from database
    let user = sqlx::query_as::<_, ManagedUser>(
        "SELECT * FROM managed_users WHERE id = ?"
    )
    .bind(user_id)
    .fetch_optional(pool.get_ref())
    .await;

    match user {
        Ok(Some(user)) => {
            // Get last 7 days of usage data, always return 7 days with actual dates
            let mut usage_data = Vec::new();
            
            for i in 0..7 {
                let date = Utc::now().date_naive() - chrono::Duration::days(6 - i);
                
                // Try to get usage for this specific date
                let time_spent = sqlx::query_scalar::<_, Option<i64>>(
                    "SELECT time_spent FROM user_time_usage WHERE user_id = ? AND date = ?"
                )
                .bind(user_id)
                .bind(date)
                .fetch_optional(pool.get_ref())
                .await
                .unwrap_or(None)
                .flatten()
                .unwrap_or(0);
                
                usage_data.push(serde_json::json!({
                    "date": date.to_string(),
                    "hours": (time_spent as f64) / 3600.0
                }));
            }

            let usage_response = serde_json::json!({
                "success": true,
                "data": usage_data,
                "username": user.username
            });
            
            Ok(HttpResponse::Ok().json(usage_response))
        }
        Ok(None) => Ok(HttpResponse::NotFound().json(serde_json::json!({
            "success": false,
            "message": "User not found"
        }))),
        Err(e) => {
            eprintln!("Database error: {}", e);
            Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "success": false,
                "message": "Database error"
            })))
        }
    }
}

async fn get_task_status(
    pool: web::Data<SqlitePool>,
    session: Session,
    scheduler: web::Data<std::sync::Arc<BackgroundScheduler>>,
) -> Result<HttpResponse> {
    if !is_authenticated(&session) {
        return Ok(HttpResponse::Unauthorized().json(serde_json::json!({
            "success": false,
            "message": "Not authenticated"
        })));
    }

    // Get actual status
    let is_running = scheduler.is_running().await;
    
    // Count managed users
    let user_count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM managed_users WHERE is_valid = 1")
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

async fn update_schedule_api(
    pool: web::Data<SqlitePool>,
    form: web::Json<ScheduleUpdateForm>,
    session: Session,
) -> Result<HttpResponse> {
    if !is_authenticated(&session) {
        return Ok(HttpResponse::Unauthorized().json(serde_json::json!({
            "success": false,
            "message": "Not authenticated"
        })));
    }

    println!("Received schedule update: user_id={}, monday={}, tuesday={}, wednesday={}, thursday={}, friday={}, saturday={}, sunday={}",
             form.user_id, form.monday, form.tuesday, form.wednesday, form.thursday, form.friday, form.saturday, form.sunday);

    // Validate schedule values (0-24 hours)
    let schedule_values = [form.monday, form.tuesday, form.wednesday, form.thursday, form.friday, form.saturday, form.sunday];
    for value in &schedule_values {
        if *value < 0.0 || *value > 24.0 {
            return Ok(HttpResponse::BadRequest().json(serde_json::json!({
                "success": false,
                "message": "Schedule values must be between 0 and 24 hours"
            })));
        }
    }

    // Update schedule in database (this will mark as not synced)
    println!("Updating database with: user_id={}, monday={}, saturday={}, sunday={}", form.user_id, form.monday, form.saturday, form.sunday);
    
    match sqlx::query(
        "UPDATE user_weekly_schedule 
         SET monday_hours=?, tuesday_hours=?, wednesday_hours=?, thursday_hours=?, friday_hours=?, saturday_hours=?, sunday_hours=?, 
             is_synced=false, last_modified=?
         WHERE user_id=?"
    )
    .bind(form.monday)
    .bind(form.tuesday) 
    .bind(form.wednesday)
    .bind(form.thursday)
    .bind(form.friday)
    .bind(form.saturday)
    .bind(form.sunday)
    .bind(Utc::now())
    .bind(form.user_id)
    .execute(pool.get_ref())
    .await {
        Ok(result) => {
            println!("Database update successful, rows affected: {}", result.rows_affected());
            
            // Verify the data was actually saved by reading it back immediately
            let verify_query = sqlx::query_as::<_, (f64, f64)>(
                "SELECT saturday_hours, sunday_hours FROM user_weekly_schedule WHERE user_id = ?"
            )
            .bind(form.user_id)
            .fetch_optional(pool.get_ref())
            .await;
            
            if let Ok(Some(verify_data)) = verify_query {
                println!("Verification read: saturday={}, sunday={}", verify_data.0, verify_data.1);
            } else {
                println!("Verification read failed or no data found");
            }
            
            Ok(HttpResponse::Ok().json(serde_json::json!({
                "success": true,
                "message": "Schedule updated successfully"
            })))
        }
        Err(e) => {
            eprintln!("Failed to update schedule: {}", e);
            Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "success": false,
                "message": format!("Failed to update schedule: {}", e)
            })))
        }
    }
}

async fn get_schedule_sync_status(
    pool: web::Data<SqlitePool>,
    path: web::Path<i64>,
    session: Session,
) -> Result<HttpResponse> {
    if !is_authenticated(&session) {
        return Ok(HttpResponse::Unauthorized().json(serde_json::json!({
            "success": false,
            "message": "Not authenticated"
        })));
    }

    let user_id = path.into_inner();
    
    // Get schedule from database
    let schedule_result = sqlx::query_as::<_, (f64, f64, f64, f64, f64, f64, f64, bool, Option<chrono::DateTime<Utc>>, Option<chrono::DateTime<Utc>>)>(
        "SELECT monday_hours, tuesday_hours, wednesday_hours, thursday_hours, friday_hours, saturday_hours, sunday_hours, 
         is_synced, last_synced, last_modified 
         FROM user_weekly_schedule WHERE user_id = ?"
    )
    .bind(user_id)
    .fetch_optional(pool.get_ref())
    .await;

    match schedule_result {
        Ok(Some(schedule)) => {
            let schedule_dict = serde_json::json!({
                "monday": schedule.0,
                "tuesday": schedule.1,
                "wednesday": schedule.2,
                "thursday": schedule.3,
                "friday": schedule.4,
                "saturday": schedule.5,
                "sunday": schedule.6
            });
            
            println!("Retrieved schedule for user {}: {:?}", user_id, schedule_dict);
            
            Ok(HttpResponse::Ok().json(serde_json::json!({
                "success": true,
                "is_synced": schedule.7,
                "schedule": schedule_dict,
                "last_synced": schedule.8.map(|dt| dt.format("%Y-%m-%d %H:%M").to_string()),
                "last_modified": schedule.9.map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
            })))
        }
        Ok(None) => {
            Ok(HttpResponse::Ok().json(serde_json::json!({
                "success": true,
                "is_synced": true,
                "schedule": null,
                "last_synced": null,
                "last_modified": null
            })))
        }
        Err(e) => {
            eprintln!("Failed to get schedule: {}", e);
            Ok(HttpResponse::InternalServerError().json(serde_json::json!({
                "success": false,
                "message": "Failed to get schedule data"
            })))
        }
    }
}

async fn get_ssh_status(session: Session) -> Result<HttpResponse> {
    if !is_authenticated(&session) {
        return Ok(HttpResponse::Unauthorized().json(serde_json::json!({
            "success": false,
            "message": "Not authenticated"
        })));
    }

    let ssh_key_exists = SSHClient::check_ssh_key_exists();
    
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "ssh_key_exists": ssh_key_exists,
        "message": if ssh_key_exists {
            "SSH keys are configured"
        } else {
            "SSH keys not found. Please configure SSH keys for passwordless authentication."
        }
    })))
}

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    // Initialize database
    let database_url = "sqlite:instance/timekpr.db";
    let pool = SqlitePool::connect(database_url).await?;
    
    // Run migrations
    sqlx::migrate!("./migrations").run(&pool).await?;
    
    // Initialize admin password
    let admin_hash = sqlx::query_scalar::<_, Option<String>>("SELECT value FROM settings WHERE key = 'admin_password_hash'")
        .fetch_optional(&pool)
        .await?;
    
    if admin_hash.is_none() {
        use argon2::{Argon2, PasswordHasher};
        use argon2::password_hash::{rand_core::OsRng, SaltString};
        
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let password_hash = argon2.hash_password("admin".as_bytes(), &salt)
            .map_err(|e| anyhow::anyhow!("Failed to hash password: {}", e))?;
        
        sqlx::query("INSERT OR REPLACE INTO settings (key, value) VALUES ('admin_password_hash', ?)")
            .bind(password_hash.to_string())
            .execute(&pool)
            .await?;
        
        println!("Admin password initialized");
    }

    // Initialize and start background scheduler
    let scheduler = std::sync::Arc::new(BackgroundScheduler::new(pool.clone()));
    scheduler.start().await;
    println!("Background scheduler started");

    println!("Server listening on 0.0.0.0:5000");
    
    // Generate a random key for sessions
    let secret_key = Key::generate();
    
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::from(scheduler.clone()))
            .wrap(
                Cors::default()
                    .allow_any_origin()
                    .allow_any_method() 
                    .allow_any_header()
                    .supports_credentials()
            )
            .wrap(SessionMiddleware::new(
                CookieSessionStore::default(),
                secret_key.clone()
            ))
            .wrap(middleware::Logger::default())
            // API endpoints only - no static file serving (frontend will be separate)
            .route("/api/login", web::post().to(login_api))
            .route("/api/logout", web::post().to(logout_api))
            .route("/api/dashboard", web::get().to(dashboard_api))
            .route("/api/admin", web::get().to(admin_api))
            .route("/api/change-password", web::post().to(change_password_api))
            .route("/api/users/add", web::post().to(add_user_api))
            .route("/api/users/validate/{id}", web::get().to(validate_user))
            .route("/api/users/delete/{id}", web::post().to(delete_user))
            .route("/api/modify-time", web::post().to(modify_time))
            .route("/api/user/{id}/usage", web::get().to(get_user_usage))
            .route("/api/schedule-sync-status/{id}", web::get().to(get_schedule_sync_status))
            .route("/api/schedule/update", web::post().to(update_schedule_api))
            .route("/api/task-status", web::get().to(get_task_status))
            .route("/api/ssh-status", web::get().to(get_ssh_status))
    })
    .bind("0.0.0.0:5000")?
    .run()
    .await?;

    Ok(())
}