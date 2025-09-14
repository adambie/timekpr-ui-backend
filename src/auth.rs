use actix_web::{HttpRequest, Result as ActixResult};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, TokenData, Validation};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String, // Subject (username)
    pub exp: usize,  // Expiration time
    pub iat: usize,  // Issued at
}

#[derive(Clone)]
pub struct JwtManager {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
}

impl JwtManager {
    pub fn new(secret: &str) -> Self {
        Self {
            encoding_key: EncodingKey::from_secret(secret.as_ref()),
            decoding_key: DecodingKey::from_secret(secret.as_ref()),
        }
    }

    pub fn generate_token(&self, username: &str) -> Result<String, jsonwebtoken::errors::Error> {
        let now = Utc::now();
        let expires_in = Duration::hours(24); // 24 hour expiration

        let claims = Claims {
            sub: username.to_string(),
            exp: (now + expires_in).timestamp() as usize,
            iat: now.timestamp() as usize,
        };

        encode(&Header::default(), &claims, &self.encoding_key)
    }

    pub fn verify_token(
        &self,
        token: &str,
    ) -> Result<TokenData<Claims>, jsonwebtoken::errors::Error> {
        decode::<Claims>(token, &self.decoding_key, &Validation::default())
    }
}

pub fn extract_token_from_header(req: &HttpRequest) -> Option<String> {
    let auth_header = req
        .headers()
        .get("Authorization")?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")?;

    // Handle case where token accidentally starts with "bearer " due to Swagger UI bug
    if auth_header.starts_with("bearer ") {
        Some(auth_header.strip_prefix("bearer ")?.to_string())
    } else {
        Some(auth_header.to_string())
    }
}

pub fn verify_jwt(req: &HttpRequest, jwt_manager: &JwtManager) -> ActixResult<Claims> {
    let token = extract_token_from_header(req)
        .ok_or_else(|| actix_web::error::ErrorUnauthorized("Missing Authorization header"))?;

    match jwt_manager.verify_token(&token) {
        Ok(token_data) => Ok(token_data.claims),
        Err(_) => Err(actix_web::error::ErrorUnauthorized("Invalid token")),
    }
}
