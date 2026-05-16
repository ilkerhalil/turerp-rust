use actix_web::body::to_bytes;
use actix_web::http::StatusCode;
use actix_web::test;
use serde_json::json;

/// Register an admin user directly and return (access_token, user_id)
pub async fn register_admin(state: &turerp::app::AppState, tenant_id: i64) -> (String, i64) {
    let username = format!("admin_{}", uuid::Uuid::new_v4().to_string());
    let user = state
        .user_service
        .get_ref()
        .create_user(turerp::CreateUser {
            username: username.clone(),
            email: format!("{}@test.com", username),
            full_name: "Admin User".to_string(),
            password: "Password123!".to_string(),
            tenant_id,
            role: Some(turerp::Role::Admin),
        })
        .await
        .unwrap();
    let tokens = state
        .jwt_service
        .get_ref()
        .generate_tokens(user.id, user.tenant_id, user.username.clone(), turerp::Role::Admin)
        .unwrap();
    (tokens.access_token, user.id)
}

/// Register a normal user via the API and return (access_token, user_id)
pub async fn register_user<S>(app: &S, tenant_id: i64) -> (String, i64)
where
    S: actix_web::dev::Service<
        actix_web::dev::ServiceRequest,
        Response = actix_web::dev::ServiceResponse<actix_web::body::EitherBody<actix_web::body::BoxBody>>,
        Error = actix_web::Error,
    >,
{
    let username = format!("user_{}", uuid::Uuid::new_v4().to_string());
    let req = test::TestRequest::post()
        .uri("/api/auth/register")
        .set_json(json!({
            "username": username,
            "email": format!("{}@test.com", username),
            "full_name": "Test User",
            "password": "Password123!",
            "tenant_id": tenant_id
        }))
        .to_request();
    let resp = test::call_service(app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED, "User registration failed");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let access_token = json["tokens"]["access_token"].as_str().unwrap().to_string();
    let user_id = json["user"]["id"].as_i64().unwrap();
    (access_token, user_id)
}
