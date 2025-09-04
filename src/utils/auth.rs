use axum::{
    extract::{Request},
    middleware::Next,
    response::{IntoResponse, Redirect, Response},
};
use tower_sessions::Session;

pub const SESSION_USER_KEY: &str = "logged_in";

pub async fn require_auth(
    session: Session,
    request: Request,
    next: Next,
) -> Response {
    match session.get::<bool>(SESSION_USER_KEY).await {
        Ok(Some(true)) => next.run(request).await,
        _ => Redirect::to("/").into_response(),
    }
}

pub async fn is_authenticated(session: &Session) -> bool {
    session.get::<bool>(SESSION_USER_KEY)
        .await
        .unwrap_or(Some(false))
        .unwrap_or(false)
}

pub async fn login_user(session: &Session) -> Result<(), tower_sessions::session::Error> {
    session.insert(SESSION_USER_KEY, true).await
}

pub async fn logout_user(session: &Session) -> Result<(), tower_sessions::session::Error> {
    session.remove::<bool>(SESSION_USER_KEY).await.map(|_| ())
}