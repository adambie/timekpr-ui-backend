use actix_web::{http::StatusCode, test};
use serde_json::json;

mod common;
use common::TestApp;

#[actix_web::test]
async fn test_add_user_success() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;

    // Login to get token
    let token = test_app.login_and_get_token().await;

    let req = test::TestRequest::post()
        .uri("/api/users/add")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "username": "testuser",
            "system_ip": "192.168.1.100"
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["success"], true);
    assert!(body["message"].as_str().unwrap().contains("testuser"));
    // SSH validation will fail in test environment, but user creation should succeed
}

#[actix_web::test]
async fn test_add_user_missing_username() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;

    let token = test_app.login_and_get_token().await;

    let req = test::TestRequest::post()
        .uri("/api/users/add")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "system_ip": "192.168.1.100"
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[actix_web::test]
async fn test_add_user_invalid_ip() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;

    let token = test_app.login_and_get_token().await;

    let req = test::TestRequest::post()
        .uri("/api/users/add")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "username": "testuser",
            "system_ip": "invalid_ip"
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["success"], true);
    // Invalid IP will cause SSH validation to fail but user will still be created
    assert!(body["message"]
        .as_str()
        .unwrap()
        .contains("validation failed"));
}

#[actix_web::test]
async fn test_add_duplicate_user() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;

    let token = test_app.login_and_get_token().await;

    // Add user first time
    let req1 = test::TestRequest::post()
        .uri("/api/users/add")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "username": "testuser",
            "system_ip": "192.168.1.100"
        }))
        .to_request();

    let resp1 = test::call_service(&app, req1).await;
    assert_eq!(resp1.status(), StatusCode::OK);

    // Try to add same user again
    let req2 = test::TestRequest::post()
        .uri("/api/users/add")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "username": "testuser",
            "system_ip": "192.168.1.100"
        }))
        .to_request();

    let resp2 = test::call_service(&app, req2).await;
    assert_eq!(resp2.status(), StatusCode::BAD_REQUEST);

    let body: serde_json::Value = test::read_body_json(resp2).await;
    assert_eq!(body["success"], false);
    assert!(body["message"].as_str().unwrap().contains("already exists"));
}

#[actix_web::test]
async fn test_remove_user_success() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;

    let token = test_app.login_and_get_token().await;
    let user_id = test_app.add_test_user(&token).await;

    let req = test::TestRequest::post()
        .uri(&format!("/api/users/delete/{}", user_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["success"], true);
    assert!(body["message"].as_str().unwrap().contains("deleted"));
}

#[actix_web::test]
async fn test_remove_nonexistent_user() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;

    let token = test_app.login_and_get_token().await;

    let req = test::TestRequest::post()
        .uri("/api/users/delete/99999")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["success"], false);
    assert!(body["message"].as_str().unwrap().contains("not found"));
}

#[actix_web::test]
async fn test_user_operations_without_auth() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;

    // Test add user without token
    let req = test::TestRequest::post()
        .uri("/api/users/add")
        .set_json(json!({
            "username": "testuser",
            "system_ip": "192.168.1.100"
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    // Test remove user without token
    let req = test::TestRequest::post()
        .uri("/api/users/delete/1")
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}
