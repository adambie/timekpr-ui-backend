use axum::{
    extract::{Path, State},
    response::{Html, Redirect, Response, IntoResponse},
    Form,
};
use serde::Deserialize;
use tower_sessions::Session;

use crate::{
    database::models::{ManagedUser, Settings},
    services::ssh::SSHClient,
    utils::auth::is_authenticated,
    AppState,
};

#[derive(Deserialize)]
pub struct AddUserForm {
    username: String,
    system_ip: String,
}

#[derive(Deserialize)]
pub struct SettingsForm {
    current_password: String,
    new_password: String,
    confirm_password: String,
}

pub async fn admin(State(state): State<AppState>, session: Session) -> Response {
    if !is_authenticated(&session).await {
        return Redirect::to("/").into_response();
    }
    
    match ManagedUser::get_all(&state.pool).await {
        Ok(users) => {
            let html = include_str!("../../templates/admin.html")
                .replace("{{ users }}", &serde_json::to_string(&users).unwrap_or_default());
            Html(html).into_response()
        }
        Err(e) => {
            tracing::error!("Database error: {}", e);
            Html("Database error").into_response()
        }
    }
}

pub async fn settings(session: Session) -> Response {
    if !is_authenticated(&session).await {
        return Redirect::to("/").into_response();
    }
    
    Html(include_str!("../../templates/settings.html")).into_response()
}

pub async fn update_settings(
    State(state): State<AppState>,
    session: Session,
    Form(form): Form<SettingsForm>,
) -> Response {
    if !is_authenticated(&session).await {
        return Redirect::to("/").into_response();
    }
    
    // Validate inputs
    if form.current_password.is_empty() || form.new_password.is_empty() || form.confirm_password.is_empty() {
        let html = include_str!("../../templates/settings.html")
            .replace("{{ error }}", "All fields are required");
        return Html(html).into_response();
    }
    
    if form.new_password != form.confirm_password {
        let html = include_str!("../../templates/settings.html")
            .replace("{{ error }}", "New passwords do not match");
        return Html(html).into_response();
    }
    
    if form.new_password.len() < 4 {
        let html = include_str!("../../templates/settings.html")
            .replace("{{ error }}", "New password must be at least 4 characters long");
        return Html(html).into_response();
    }
    
    // Check current password
    match Settings::check_admin_password(&state.pool, &form.current_password).await {
        Ok(true) => {
            // Update password
            match Settings::set_admin_password(&state.pool, &form.new_password).await {
                Ok(()) => Redirect::to("/settings").into_response(),
                Err(e) => {
                    tracing::error!("Failed to update password: {}", e);
                    let html = include_str!("../../templates/settings.html")
                        .replace("{{ error }}", "Failed to update password");
                    Html(html).into_response()
                }
            }
        }
        Ok(false) => {
            let html = include_str!("../../templates/settings.html")
                .replace("{{ error }}", "Current password is incorrect");
            Html(html).into_response()
        }
        Err(e) => {
            tracing::error!("Database error during password check: {}", e);
            let html = include_str!("../../templates/settings.html")
                .replace("{{ error }}", "System error. Please try again.");
            Html(html).into_response()
        }
    }
}

pub async fn add_user(
    State(state): State<AppState>,
    session: Session,
    Form(form): Form<AddUserForm>,
) -> Response {
    if !is_authenticated(&session).await {
        return Redirect::to("/").into_response();
    }
    
    if form.username.is_empty() || form.system_ip.is_empty() {
        return Redirect::to("/admin").into_response();
    }
    
    // Check if user already exists
    match ManagedUser::find_by_username_and_ip(&state.pool, &form.username, &form.system_ip).await {
        Ok(Some(_)) => {
            // User already exists
            return Redirect::to("/admin").into_response();
        }
        Ok(None) => {
            // Create new user
            match ManagedUser::create(&state.pool, &form.username, &form.system_ip).await {
                Ok(user_id) => {
                    // Validate user with SSH
                    let ssh_client = SSHClient::new(form.system_ip.clone());
                    match ssh_client.validate_user(&form.username).await {
                        Ok((is_valid, _message, config)) => {
                            let config_json = config.as_ref().map(|c| serde_json::to_string(c).unwrap_or_default());
                            let _ = ManagedUser::update_validation(&state.pool, user_id, is_valid, config_json.as_deref()).await;
                            
                            // Add today's usage if valid
                            if is_valid && config.is_some() {
                                let today = chrono::Utc::now().date_naive();
                                if let Some(time_spent) = config.as_ref().unwrap().get("TIME_SPENT_DAY").and_then(|v| v.as_i64()) {
                                    let _ = crate::database::models::UserTimeUsage::upsert(&state.pool, user_id, today, time_spent).await;
                                }
                            }
                        }
                        Err(e) => {
                            tracing::error!("SSH validation error: {}", e);
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to create user: {}", e);
                }
            }
        }
        Err(e) => {
            tracing::error!("Database error: {}", e);
        }
    }
    
    Redirect::to("/admin").into_response()
}

pub async fn validate_user(
    State(state): State<AppState>,
    session: Session,
    Path(user_id): Path<i64>,
) -> Response {
    if !is_authenticated(&session).await {
        return Redirect::to("/").into_response();
    }
    
    // Get user
    match sqlx::query_as::<_, ManagedUser>(
        "SELECT id, username, system_ip, is_valid, date_added, last_checked, last_config, pending_time_adjustment, pending_time_operation FROM managed_user WHERE id = ?"
    )
    .bind(user_id)
    .fetch_optional(&state.pool)
    .await {
        Ok(Some(user)) => {
            let ssh_client = SSHClient::new(user.system_ip.clone());
            match ssh_client.validate_user(&user.username).await {
                Ok((is_valid, _message, config)) => {
                    let config_json = config.as_ref().map(|c| serde_json::to_string(c).unwrap_or_default());
                    let _ = ManagedUser::update_validation(&state.pool, user_id, is_valid, config_json.as_deref()).await;
                    
                    if is_valid && config.is_some() {
                        let today = chrono::Utc::now().date_naive();
                        if let Some(time_spent) = config.as_ref().unwrap().get("TIME_SPENT_DAY").and_then(|v| v.as_i64()) {
                            let _ = crate::database::models::UserTimeUsage::upsert(&state.pool, user_id, today, time_spent).await;
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("SSH validation error: {}", e);
                }
            }
        }
        Ok(None) => {
            tracing::warn!("User {} not found", user_id);
        }
        Err(e) => {
            tracing::error!("Database error: {}", e);
        }
    }
    
    Redirect::to("/admin").into_response()
}

pub async fn delete_user(
    State(state): State<AppState>,
    session: Session,
    Path(user_id): Path<i64>,
) -> Response {
    if !is_authenticated(&session).await {
        return Redirect::to("/").into_response();
    }
    
    match ManagedUser::delete_by_id(&state.pool, user_id).await {
        Ok(()) => {},
        Err(e) => {
            tracing::error!("Failed to delete user: {}", e);
        }
    }
    
    Redirect::to("/admin").into_response()
}