use actix_web::{web, App, HttpServer, middleware::Logger};
use actix_cors::Cors;
use sqlx::SqlitePool;
use utoipa::OpenApi;

mod ssh;
mod scheduler;
mod models;
mod auth;
mod openapi_config;
mod handlers;
mod middleware;
mod config;
mod services;
mod repositories;

use scheduler::BackgroundScheduler;
use auth::JwtManager;
use openapi_config::configure_openapi;
use config::ApiDoc;
use services::{ScheduleService, UserService, TimeService};
use repositories::{SqliteScheduleRepository, SqliteUserRepository, SqliteUsageRepository};
use std::sync::Arc;

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    // Initialize database
    let database_url = "sqlite:instance/timekpr.db";
    let pool = SqlitePool::connect(database_url).await?;
    
    // Run migrations (disabled - already applied manually)
    // sqlx::migrate!("./migrations").run(&pool).await?;
    
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
    }

    // Initialize repositories
    let schedule_repository = Arc::new(SqliteScheduleRepository::new(pool.clone()));
    let user_repository = Arc::new(SqliteUserRepository::new(pool.clone()));
    let usage_repository = Arc::new(SqliteUsageRepository::new(pool.clone()));

    // Initialize services with dependency injection
    let schedule_service = web::Data::new(ScheduleService::new(schedule_repository));
    let user_service = web::Data::new(UserService::new(user_repository.clone()));
    let time_service = web::Data::new(TimeService::new(user_repository, usage_repository));

    // Initialize and start background scheduler
    let scheduler = std::sync::Arc::new(BackgroundScheduler::new(pool.clone()));
    scheduler.start().await;

    // Initialize JWT manager with secret key
    let jwt_secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "your-secret-key-change-in-production".to_string());
    let jwt_manager = web::Data::new(JwtManager::new(&jwt_secret));

    println!("TimeKpr UI Server listening on http://localhost:5000");
    println!("📚 API Documentation: http://localhost:5000/swagger-ui/");
    
    // Configure OpenAPI spec with Bearer auth (do this once, outside the closure)
    let openapi_spec = configure_openapi(ApiDoc::openapi());
    
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::from(scheduler.clone()))
            .app_data(jwt_manager.clone())
            .app_data(schedule_service.clone())
            .app_data(user_service.clone())
            .app_data(time_service.clone())
            .wrap(
                Cors::default()
                    .allow_any_origin()
                    .allow_any_method() 
                    .allow_any_header()
                    .supports_credentials()
            )
            .wrap(Logger::default())
            // Swagger UI for API documentation
            .service(
                utoipa_swagger_ui::SwaggerUi::new("/swagger-ui/{_:.*}")
                    .url("/api-docs/openapi.json", openapi_spec.clone())
            )
            // API endpoints only - no static file serving (frontend will be separate)
            .route("/api/login", web::post().to(handlers::login_api))
            .route("/api/logout", web::post().to(handlers::logout_api))
            .route("/api/dashboard", web::get().to(handlers::dashboard_api))
            .route("/api/admin", web::get().to(handlers::admin_api))
            .route("/api/change-password", web::post().to(handlers::change_password_api))
            .route("/api/users/add", web::post().to(handlers::add_user_api))
            .route("/api/users/validate/{id}", web::get().to(handlers::validate_user))
            .route("/api/users/delete/{id}", web::post().to(handlers::delete_user))
            .route("/api/modify-time", web::post().to(handlers::modify_time))
            .route("/api/user/{id}/usage", web::get().to(handlers::get_user_usage))
            .route("/api/schedule-sync-status/{id}", web::get().to(handlers::get_schedule_sync_status))
            .route("/api/schedule/update", web::post().to(handlers::update_schedule_api))
            .route("/api/task-status", web::get().to(handlers::get_task_status))
            .route("/api/ssh-status", web::get().to(handlers::get_ssh_status))
    })
    .bind("0.0.0.0:5000")?
    .run()
    .await?;

    Ok(())
}