use actix_web::{http::StatusCode, test};
use serde_json::json;

mod common;
use common::TestApp;

#[actix_web::test]
async fn test_login_success() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;

    let req = test::TestRequest::post()
        .uri("/api/login")
        .set_json(json!({
            "username": "admin",
            "password": "admin"
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["success"], true);
    assert!(body["token"].is_string());
    assert!(!body["token"].as_str().unwrap().is_empty());
}

#[actix_web::test]
async fn test_login_invalid_credentials() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;

    let req = test::TestRequest::post()
        .uri("/api/login")
        .set_json(json!({
            "username": "admin",
            "password": "wrong_password"
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["success"], false);
    assert!(body["message"].as_str().unwrap().contains("Invalid"));
}

#[actix_web::test]
async fn test_login_missing_username() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;

    let req = test::TestRequest::post()
        .uri("/api/login")
        .set_json(json!({
            "password": "admin"
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[actix_web::test]
async fn test_login_empty_request_body() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;

    let req = test::TestRequest::post()
        .uri("/api/login")
        .set_json(json!({}))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[actix_web::test]
async fn test_protected_endpoint_without_token() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;

    let req = test::TestRequest::post()
        .uri("/api/users/add")
        .set_json(json!({
            "username": "testuser",
            "system_ip": "192.168.1.100"
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn test_protected_endpoint_with_invalid_token() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;

    let req = test::TestRequest::post()
        .uri("/api/users/add")
        .insert_header(("Authorization", "Bearer invalid_token"))
        .set_json(json!({
            "username": "testuser",
            "system_ip": "192.168.1.100"
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}
