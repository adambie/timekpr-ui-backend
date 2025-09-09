use actix_web::{test, http::StatusCode};
use serde_json::json;

mod common;
use common::TestApp;

#[actix_web::test]
async fn test_update_schedule_success() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;
    
    let token = test_app.login_and_get_token().await;
    let user_id = test_app.add_test_user(&token).await;

    let req = test::TestRequest::post()
        .uri("/api/schedule/update")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "user_id": user_id,
            "monday": 2.5,
            "tuesday": 3.0,
            "wednesday": 2.0,
            "thursday": 3.5,
            "friday": 4.0,
            "saturday": 5.0,
            "sunday": 4.5
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["success"], true);
    assert!(body["message"].as_str().unwrap().contains("updated"));
}

#[actix_web::test]
async fn test_update_schedule_invalid_hours() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;
    
    let token = test_app.login_and_get_token().await;
    let user_id = test_app.add_test_user(&token).await;

    // Test negative hours
    let req = test::TestRequest::post()
        .uri("/api/schedule/update")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "user_id": user_id,
            "monday": -1.0,
            "tuesday": 3.0,
            "wednesday": 2.0,
            "thursday": 3.5,
            "friday": 4.0,
            "saturday": 5.0,
            "sunday": 4.5
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["success"], false);
    assert!(body["message"].as_str().unwrap().contains("between 0 and 24"));
}

#[actix_web::test]
async fn test_update_schedule_hours_over_limit() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;
    
    let token = test_app.login_and_get_token().await;
    let user_id = test_app.add_test_user(&token).await;

    // Test hours over 24
    let req = test::TestRequest::post()
        .uri("/api/schedule/update")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "user_id": user_id,
            "monday": 25.0,
            "tuesday": 3.0,
            "wednesday": 2.0,
            "thursday": 3.5,
            "friday": 4.0,
            "saturday": 5.0,
            "sunday": 4.5
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["success"], false);
    assert!(body["message"].as_str().unwrap().contains("between 0 and 24"));
}

#[actix_web::test]
async fn test_update_schedule_nonexistent_user() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;
    
    let token = test_app.login_and_get_token().await;

    let req = test::TestRequest::post()
        .uri("/api/schedule/update")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "user_id": 99999,
            "monday": 2.5,
            "tuesday": 3.0,
            "wednesday": 2.0,
            "thursday": 3.5,
            "friday": 4.0,
            "saturday": 5.0,
            "sunday": 4.5
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["success"], false);
    assert!(body["message"].as_str().unwrap().contains("Database error"));
}

#[actix_web::test]
async fn test_get_schedule_success() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;
    
    let token = test_app.login_and_get_token().await;
    let user_id = test_app.add_test_user(&token).await;

    // First update the schedule
    let update_req = test::TestRequest::post()
        .uri("/api/schedule/update")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "user_id": user_id,
            "monday": 2.5,
            "tuesday": 3.0,
            "wednesday": 2.0,
            "thursday": 3.5,
            "friday": 4.0,
            "saturday": 5.0,
            "sunday": 4.5
        }))
        .to_request();

    test::call_service(&app, update_req).await;

    // Now get the schedule
    let get_req = test::TestRequest::get()
        .uri(&format!("/api/schedule/{}", user_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, get_req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["success"], true);
    assert!(body["schedule"].is_object());
    assert_eq!(body["schedule"]["monday"], 2.5);
    assert_eq!(body["schedule"]["tuesday"], 3.0);
    assert_eq!(body["schedule"]["sunday"], 4.5);
}

#[actix_web::test]
async fn test_get_schedule_nonexistent_user() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;
    
    let token = test_app.login_and_get_token().await;

    let req = test::TestRequest::get()
        .uri("/api/schedule/99999")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["success"], true);
    assert!(body["schedule"].is_null()); // No schedule for nonexistent user
}

#[actix_web::test]
async fn test_schedule_operations_without_auth() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;

    // Test update schedule without token
    let req = test::TestRequest::post()
        .uri("/api/schedule/update")
        .set_json(json!({
            "user_id": 1,
            "monday": 2.5,
            "tuesday": 3.0,
            "wednesday": 2.0,
            "thursday": 3.5,
            "friday": 4.0,
            "saturday": 5.0,
            "sunday": 4.5
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    // Test get schedule without token
    let req = test::TestRequest::get()
        .uri("/api/schedule/1")
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn test_update_schedule_missing_day() {
    let test_app = TestApp::new().await;
    let app = test::init_service(test_app.create_app()).await;
    
    let token = test_app.login_and_get_token().await;
    let user_id = test_app.add_test_user(&token).await;

    // Missing sunday field
    let req = test::TestRequest::post()
        .uri("/api/schedule/update")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "user_id": user_id,
            "monday": 2.5,
            "tuesday": 3.0,
            "wednesday": 2.0,
            "thursday": 3.5,
            "friday": 4.0,
            "saturday": 5.0
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}