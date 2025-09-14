use actix_web::{http::StatusCode, test};

mod common;
use common::TestApp;

#[actix_web::test]
async fn test_dashboard_success() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;

    let token = test_app.login_and_get_token().await;

    // Add a test user - it will fail SSH validation in test environment
    let _user_id = test_app.add_test_user(&token).await;

    let req = test::TestRequest::get()
        .uri("/api/dashboard")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["success"], true);
    assert!(body["users"].is_array());

    // Dashboard only shows valid users - our test user will fail SSH validation
    let users = body["users"].as_array().unwrap();
    assert!(users.is_empty()); // No valid users in test environment
}

#[actix_web::test]
async fn test_dashboard_empty_users() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;

    let token = test_app.login_and_get_token().await;

    let req = test::TestRequest::get()
        .uri("/api/dashboard")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["success"], true);
    assert!(body["users"].is_array());

    let users = body["users"].as_array().unwrap();
    assert!(users.is_empty());
}

#[actix_web::test]
async fn test_dashboard_without_auth() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;

    let req = test::TestRequest::get().uri("/api/dashboard").to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn test_dashboard_with_invalid_token() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;

    let req = test::TestRequest::get()
        .uri("/api/dashboard")
        .insert_header(("Authorization", "Bearer invalid_token"))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn test_dashboard_response_structure() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;

    let token = test_app.login_and_get_token().await;

    let req = test::TestRequest::get()
        .uri("/api/dashboard")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    let body: serde_json::Value = test::read_body_json(resp).await;

    // Verify response structure is correct
    assert!(body.get("success").is_some());
    assert_eq!(body["success"], true);
    assert!(body.get("users").is_some());
    assert!(body["users"].is_array());
}
