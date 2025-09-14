use crate::auth::{verify_jwt, JwtManager};
use actix_web;

pub fn authenticate_request(
    req: &actix_web::HttpRequest,
    jwt_manager: &JwtManager,
) -> Result<(), actix_web::Error> {
    match verify_jwt(req, jwt_manager) {
        Ok(_claims) => Ok(()),
        Err(e) => Err(e),
    }
}
