use actix_web::{http::StatusCode, test};
use serde_json::json;

mod common;
use common::TestApp;

#[actix_web::test]
async fn test_modify_time_add_success() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;

    let token = test_app.login_and_get_token().await;
    let user_id = test_app.add_test_user(&token).await;

    let req = test::TestRequest::post()
        .uri("/api/modify-time")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "user_id": user_id,
            "operation": "+",
            "seconds": 3600
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["success"], true);
    // Should indicate it's queued for sync since we don't have real SSH
    assert!(body.get("pending").is_some() || body["message"].as_str().unwrap().contains("queued"));
}

#[actix_web::test]
async fn test_modify_time_subtract_success() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;

    let token = test_app.login_and_get_token().await;
    let user_id = test_app.add_test_user(&token).await;

    let req = test::TestRequest::post()
        .uri("/api/modify-time")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "user_id": user_id,
            "operation": "-",
            "seconds": 1800
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["success"], true);
}

#[actix_web::test]
async fn test_modify_time_invalid_operation() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;

    let token = test_app.login_and_get_token().await;
    let user_id = test_app.add_test_user(&token).await;

    let req = test::TestRequest::post()
        .uri("/api/modify-time")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "user_id": user_id,
            "operation": "*",
            "seconds": 3600
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["success"], false);
    assert!(body["message"]
        .as_str()
        .unwrap()
        .contains("must be '+' or '-'"));
}

#[actix_web::test]
async fn test_modify_time_zero_seconds() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;

    let token = test_app.login_and_get_token().await;
    let user_id = test_app.add_test_user(&token).await;

    let req = test::TestRequest::post()
        .uri("/api/modify-time")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "user_id": user_id,
            "operation": "+",
            "seconds": 0
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["success"], false);
    assert!(body["message"]
        .as_str()
        .unwrap()
        .contains("must be positive"));
}

#[actix_web::test]
async fn test_modify_time_negative_seconds() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;

    let token = test_app.login_and_get_token().await;
    let user_id = test_app.add_test_user(&token).await;

    let req = test::TestRequest::post()
        .uri("/api/modify-time")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "user_id": user_id,
            "operation": "+",
            "seconds": -1800
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["success"], false);
    assert!(body["message"]
        .as_str()
        .unwrap()
        .contains("must be positive"));
}

#[actix_web::test]
async fn test_modify_time_nonexistent_user() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;

    let token = test_app.login_and_get_token().await;

    let req = test::TestRequest::post()
        .uri("/api/modify-time")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "user_id": 99999,
            "operation": "+",
            "seconds": 3600
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[actix_web::test]
async fn test_modify_time_without_auth() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;

    let req = test::TestRequest::post()
        .uri("/api/modify-time")
        .set_json(json!({
            "user_id": 1,
            "operation": "+",
            "seconds": 3600
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn test_modify_time_missing_fields() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;

    let token = test_app.login_and_get_token().await;

    // Missing operation
    let req = test::TestRequest::post()
        .uri("/api/modify-time")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "user_id": 1,
            "seconds": 3600
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}
