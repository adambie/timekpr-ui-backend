use axum::{
    response::{Html, IntoResponse, Redirect},
    routing::get,
    Router,
};
use sqlx::SqlitePool;
use tower_http::services::ServeDir;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize database
    let database_url = "sqlite:instance/timekpr.db";
    let pool = SqlitePool::connect(database_url).await?;
    
    // Run migrations
    sqlx::migrate!("./migrations").run(&pool).await?;
    
    // Initialize admin password
    let admin_hash = sqlx::query_scalar::<_, String>("SELECT value FROM settings WHERE key = 'admin_password_hash'")
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

    let app = Router::new()
        .nest_service("/static", ServeDir::new("static"))
        .route("/", get(|| async { Html(include_str!("../templates/login.html")) }))
        .route("/dashboard", get(|| async { Html(include_str!("../templates/dashboard.html")) }))
        .route("/admin", get(|| async { Html(include_str!("../templates/admin.html")) }))
        .route("/settings", get(|| async { Html(include_str!("../templates/settings.html")) }));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:5000").await?;
    println!("Server listening on 0.0.0.0:5000");
    
    axum::serve(listener, app).await?;
    Ok(())
}