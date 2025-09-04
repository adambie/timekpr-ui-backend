use axum::{
    extract::State,
    response::{Html, Redirect, Response, IntoResponse},
    Form,
};
use serde::Deserialize;
use tower_sessions::Session;

use crate::{database::models::Settings, utils::auth::{login_user, logout_user}, AppState};

#[derive(Deserialize)]
pub struct LoginForm {
    username: String,
    password: String,
}

pub async fn login_page() -> Html<&'static str> {
    Html(include_str!("../../templates/login.html"))
}

pub async fn login(
    State(state): State<AppState>,
    session: Session,
    Form(form): Form<LoginForm>,
) -> Response {
    if form.username == state.config.admin_username {
        match Settings::check_admin_password(&state.pool, &form.password).await {
            Ok(true) => {
                if let Err(e) = login_user(&session).await {
                    tracing::error!("Failed to set session: {}", e);
                }
                return Redirect::to("/dashboard").into_response();
            }
            Ok(false) => {
                // Invalid password - show login page with error
                let html = include_str!("../../templates/login.html")
                    .replace("{{ error }}", "Invalid credentials. Please try again.");
                return Html(html).into_response();
            }
            Err(e) => {
                tracing::error!("Database error during login: {}", e);
                let html = include_str!("../../templates/login.html")
                    .replace("{{ error }}", "System error. Please try again.");
                return Html(html).into_response();
            }
        }
    } else {
        // Invalid username - show login page with error
        let html = include_str!("../../templates/login.html")
            .replace("{{ error }}", "Invalid credentials. Please try again.");
        return Html(html).into_response();
    }
}

pub async fn logout(session: Session) -> Redirect {
    if let Err(e) = logout_user(&session).await {
        tracing::error!("Failed to clear session: {}", e);
    }
    Redirect::to("/")
}