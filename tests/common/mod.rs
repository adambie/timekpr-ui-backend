use actix_web::{web, App, test};
use sqlx::{SqlitePool, sqlite::SqlitePoolOptions};
use std::sync::Arc;
use tempfile::TempDir;
use timekpr_ui_rust::{
    repositories::{
        user_repository::SqliteUserRepository,
        schedule_repository::SqliteScheduleRepository,
    },
    services::{
        user_service::UserService,
        schedule_service::ScheduleService,
        time_service::TimeService,
    },
    handlers,
    auth::JwtManager,
    models::ManagedUser,
};

pub struct TestApp {
    pub pool: SqlitePool,
    pub jwt_manager: JwtManager,
    #[allow(dead_code)]
    pub temp_dir: TempDir,
}

impl TestApp {
    async fn init_admin_password(pool: &SqlitePool) {
        use argon2::{Argon2, PasswordHasher};
        use argon2::password_hash::{rand_core::OsRng, SaltString};
        
        // Hash "admin" password
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let password_hash = argon2.hash_password(b"admin", &salt).unwrap();
        
        sqlx::query(
            "INSERT OR REPLACE INTO settings (key, value) VALUES ('admin_password_hash', ?)"
        )
        .bind(password_hash.to_string())
        .execute(pool)
        .await
        .expect("Failed to initialize admin password");
    }

    pub async fn new() -> Self {
        // Create temporary database
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let db_path = temp_dir.path().join("test.db");
        let database_url = format!("sqlite://{}?mode=rwc", db_path.display());

        // Create connection pool
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect(&database_url)
            .await
            .expect("Failed to create database pool");

        // Run migrations
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("Failed to run migrations");

        // Initialize default admin password (admin/admin for testing)
        Self::init_admin_password(&pool).await;

        let jwt_manager = JwtManager::new("test_secret_key");

        Self {
            pool,
            jwt_manager,
            temp_dir,
        }
    }

    pub fn create_app(&self) -> actix_web::App<
        impl actix_web::dev::ServiceFactory<
            actix_web::dev::ServiceRequest,
            Config = (),
            Response = actix_web::dev::ServiceResponse,
            Error = actix_web::Error,
            InitError = (),
        >,
    > {
        // Initialize repositories
        let user_repository = Arc::new(SqliteUserRepository::new(self.pool.clone()));
        let schedule_repository = Arc::new(SqliteScheduleRepository::new(self.pool.clone()));

        // Initialize services
        let user_service = web::Data::new(UserService::new(user_repository.clone()));
        let schedule_service = web::Data::new(ScheduleService::new(schedule_repository));
        let time_service = web::Data::new(TimeService::new(user_repository));
        let jwt_manager = web::Data::new(self.jwt_manager.clone());

        App::new()
            .app_data(user_service)
            .app_data(schedule_service)
            .app_data(time_service)
            .app_data(jwt_manager)
            .app_data(web::Data::new(self.pool.clone()))
            .route("/api/login", web::post().to(handlers::auth::login_api))
            .route("/api/dashboard", web::get().to(handlers::dashboard::dashboard_api))
            .route("/api/users/add", web::post().to(handlers::users::add_user_api))
            .route("/api/users/delete/{id}", web::post().to(handlers::users::delete_user))
            .route("/api/modify-time", web::post().to(handlers::time::modify_time))
            .route("/api/schedule/update", web::post().to(handlers::schedule::update_schedule_api))
            .route("/api/schedule/{id}", web::get().to(handlers::schedule::get_schedule_sync_status))
    }

    pub async fn login_and_get_token(&self) -> String {
        let app = test::init_service(self.create_app()).await;

        let login_req = test::TestRequest::post()
            .uri("/api/login")
            .set_json(serde_json::json!({
                "username": "admin",
                "password": "admin"
            }))
            .to_request();

        let resp = test::call_service(&app, login_req).await;
        let body: serde_json::Value = test::read_body_json(resp).await;
        
        body["token"].as_str().unwrap().to_string()
    }

    pub async fn add_test_user(&self, token: &str) -> i64 {
        let app = test::init_service(self.create_app()).await;

        let add_user_req = test::TestRequest::post()
            .uri("/api/users/add")
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .set_json(serde_json::json!({
                "username": "testuser",
                "system_ip": "192.168.1.100"
            }))
            .to_request();

        let _resp = test::call_service(&app, add_user_req).await;
        
        // Query the database directly to get the user ID since the API doesn't return it
        let user = sqlx::query_as::<_, ManagedUser>(
            "SELECT * FROM managed_users WHERE username = 'testuser' AND system_ip = '192.168.1.100'"
        )
        .fetch_one(&self.pool)
        .await
        .expect("Failed to fetch created user");
        
        user.id
    }
}