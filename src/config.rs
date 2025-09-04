use std::net::SocketAddr;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub database_url: String,
    pub bind_address: SocketAddr,
    pub session_secret: String,
    pub admin_username: String,
}

impl AppConfig {
    pub fn from_env() -> Self {
        dotenvy::dotenv().ok();
        
        let host = std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
        let port = std::env::var("PORT")
            .unwrap_or_else(|_| "5000".to_string())
            .parse::<u16>()
            .unwrap_or(5000);
        
        let bind_address = format!("{}:{}", host, port)
            .parse()
            .expect("Invalid bind address");
        
        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "sqlite:timekpr.db".to_string());
        
        let session_secret = std::env::var("SESSION_SECRET")
            .unwrap_or_else(|_| {
                use rand::Rng;
                let mut rng = rand::thread_rng();
                let bytes: [u8; 32] = rng.gen();
                use base64::Engine;
                base64::engine::general_purpose::STANDARD.encode(bytes)
            });
        
        let admin_username = std::env::var("ADMIN_USERNAME")
            .unwrap_or_else(|_| "admin".to_string());
        
        Self {
            database_url,
            bind_address,
            session_secret,
            admin_username,
        }
    }
}