//! Files API Integration Tests

use actix_web::{
    body::to_bytes,
    http::{Method, StatusCode},
    test,
};

mod common;
use common::*;

#[actix_web::test]
async fn test_upload_and_list_files() {
    let app_state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&app_state)).await;
    let (token, _) = register_admin(&app_state, 1).await;

    // Upload file
    let boundary = "----WebKitFormBoundary7MA4YWxkTrZu0gW";
    let body = format!(
        "--{}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"test.txt\"\r\nContent-Type: text/plain\r\n\r\nHello World\r\n--{}--\r\n",
        boundary, boundary
    );
    let upload_req = test::TestRequest::post()
        .uri("/api/v1/files")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .insert_header((
            "Content-Type",
            format!("multipart/form-data; boundary={}", boundary),
        ))
        .set_payload(body)
        .to_request();
    let resp = test::call_service(&app, upload_req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    // List files
    let list_req = test::TestRequest::get()
        .uri("/api/v1/files")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, list_req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json.is_array());
}

#[actix_web::test]
async fn test_list_files_requires_auth() {
    let app_state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&app_state)).await;

    let req = test::TestRequest::get().uri("/api/v1/files").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn test_file_tenant_isolation() {
    let app_state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&app_state)).await;
    let (token1, _) = register_admin(&app_state, 1).await;
    let (token2, _) = register_admin(&app_state, 2).await;

    // Upload file in tenant 1
    let boundary = "----WebKitFormBoundary7MA4YWxkTrZu0gW";
    let body = format!(
        "--{}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"tenant1.txt\"\r\nContent-Type: text/plain\r\n\r\nTenant 1 file\r\n--{}--\r\n",
        boundary, boundary
    );
    let upload_req = test::TestRequest::post()
        .uri("/api/v1/files")
        .insert_header(("Authorization", format!("Bearer {}", token1)))
        .insert_header((
            "Content-Type",
            format!("multipart/form-data; boundary={}", boundary),
        ))
        .set_payload(body)
        .to_request();
    let resp = test::call_service(&app, upload_req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    // Tenant 2 should see empty list
    let list_req = test::TestRequest::get()
        .uri("/api/v1/files")
        .insert_header(("Authorization", format!("Bearer {}", token2)))
        .to_request();
    let resp = test::call_service(&app, list_req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let files = json.as_array().unwrap();
    for file in files {
        assert_ne!(file["tenant_id"], 1);
    }
}

#[actix_web::test]
async fn test_download_file() {
    let app_state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&app_state)).await;
    let (token, _) = register_admin(&app_state, 1).await;

    // Upload file
    let boundary = "----WebKitFormBoundary7MA4YWxkTrZu0gW";
    let body = format!(
        "--{}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"test.txt\"\r\nContent-Type: text/plain\r\n\r\nHello World\r\n--{}--\r\n",
        boundary, boundary
    );
    let upload_req = test::TestRequest::post()
        .uri("/api/v1/files")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .insert_header((
            "Content-Type",
            format!("multipart/form-data; boundary={}", boundary),
        ))
        .set_payload(body)
        .to_request();
    let resp = test::call_service(&app, upload_req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let upload_body = to_bytes(resp.into_body()).await.unwrap();
    let upload_json: serde_json::Value = serde_json::from_slice(&upload_body).unwrap();
    let file_id = upload_json["id"].as_i64().unwrap();

    // Download file
    let download_req = auth_request(
        Method::GET,
        &format!("/api/v1/files/{}/download", file_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, download_req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    assert_eq!(body, "Hello World".as_bytes());
}

#[actix_web::test]
async fn test_delete_file() {
    let app_state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&app_state)).await;
    let (token, _) = register_admin(&app_state, 1).await;

    // Upload file
    let boundary = "----WebKitFormBoundary7MA4YWxkTrZu0gW";
    let body = format!(
        "--{}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"delete_me.txt\"\r\nContent-Type: text/plain\r\n\r\nDelete me\r\n--{}--\r\n",
        boundary, boundary
    );
    let upload_req = test::TestRequest::post()
        .uri("/api/v1/files")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .insert_header((
            "Content-Type",
            format!("multipart/form-data; boundary={}", boundary),
        ))
        .set_payload(body)
        .to_request();
    let resp = test::call_service(&app, upload_req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let upload_body = to_bytes(resp.into_body()).await.unwrap();
    let upload_json: serde_json::Value = serde_json::from_slice(&upload_body).unwrap();
    let file_id = upload_json["id"].as_i64().unwrap();

    // Delete file
    let delete_req = auth_request(
        Method::DELETE,
        &format!("/api/v1/files/{}", file_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, delete_req).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // Verify file is gone
    let get_req =
        auth_request(Method::GET, &format!("/api/v1/files/{}", file_id), &token).to_request();
    let resp = test::call_service(&app, get_req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[actix_web::test]
async fn test_get_file_metadata() {
    let app_state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&app_state)).await;
    let (token, _) = register_admin(&app_state, 1).await;

    // Upload file
    let boundary = "----WebKitFormBoundary7MA4YWxkTrZu0gW";
    let body = format!(
        "--{}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"meta.txt\"\r\nContent-Type: text/plain\r\n\r\nMetadata test\r\n--{}--\r\n",
        boundary, boundary
    );
    let upload_req = test::TestRequest::post()
        .uri("/api/v1/files")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .insert_header((
            "Content-Type",
            format!("multipart/form-data; boundary={}", boundary),
        ))
        .set_payload(body)
        .to_request();
    let resp = test::call_service(&app, upload_req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let upload_body = to_bytes(resp.into_body()).await.unwrap();
    let upload_json: serde_json::Value = serde_json::from_slice(&upload_body).unwrap();
    let file_id = upload_json["id"].as_i64().unwrap();

    // Get metadata
    let meta_req =
        auth_request(Method::GET, &format!("/api/v1/files/{}", file_id), &token).to_request();
    let resp = test::call_service(&app, meta_req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["original_filename"], "meta.txt");
    assert_eq!(json["content_type"], "text/plain");
}
