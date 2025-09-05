use utoipa::openapi::OpenApi;
use utoipa::openapi::security::{SecurityScheme, HttpAuthScheme, HttpBuilder};
use std::collections::BTreeMap;

pub fn configure_openapi(mut openapi: OpenApi) -> OpenApi {
    // Add Bearer token security scheme (HTTP Bearer type, not ApiKey)
    let mut security_schemes = BTreeMap::new();
    security_schemes.insert(
        "bearer_auth".to_string(),
        SecurityScheme::Http(
            HttpBuilder::new()
                .scheme(HttpAuthScheme::Bearer)
                .bearer_format("JWT")
                .description(Some("JWT token authorization"))
                .build()
        ),
    );
    
    // Add security schemes to existing components
    if let Some(components) = openapi.components.as_mut() {
        components.security_schemes = security_schemes;
    }
    
    // Add global security requirement (applies to all endpoints except those with security() override)
    openapi.security = Some(vec![
        utoipa::openapi::security::SecurityRequirement::new("bearer_auth", Vec::<String>::new())
    ]);
    
    openapi
}