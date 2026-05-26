//! Currency CRUD Integration Tests

use actix_web::{body::to_bytes, http::StatusCode, test};
use serde_json::json;

mod common;
use common::*;

// ============================================================================
// Currency CRUD Tests
// ============================================================================

#[actix_web::test]
async fn test_create_currency_success() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(actix_web::http::Method::POST, "/api/currencies", &token)
        .set_json(json!({
            "code": "USD",
            "name": "US Dollar",
            "symbol": "$",
            "decimal_places": 2,
            "is_active": true,
            "is_base": false
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["code"], "USD");
    assert_eq!(json["name"], "US Dollar");
    assert_eq!(json["symbol"], "$");
    assert!(json["id"].is_number());
}

#[actix_web::test]
async fn test_list_currencies_paginated() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    for code in [&"USD", &"EUR", &"GBP"] {
        let req = auth_request(actix_web::http::Method::POST, "/api/currencies", &token)
            .set_json(json!({
                "code": code,
                "name": format!("{} Currency", code),
                "symbol": code.chars().next().unwrap().to_string(),
                "decimal_places": 2,
                "is_active": true,
                "is_base": false
            }))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::CREATED);
    }

    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/currencies?page=1&per_page=2",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["items"].as_array().unwrap().len(), 2);
    assert_eq!(json["total"], 3);
}

#[actix_web::test]
async fn test_list_currencies_active_only() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req = auth_request(actix_web::http::Method::POST, "/api/currencies", &token)
        .set_json(json!({
            "code": "USD",
            "name": "US Dollar",
            "symbol": "$",
            "is_active": true,
            "is_base": false
        }))
        .to_request();
    test::call_service(&app, req).await;

    let req = auth_request(actix_web::http::Method::POST, "/api/currencies", &token)
        .set_json(json!({
            "code": "EUR",
            "name": "Euro",
            "symbol": "€",
            "is_active": false,
            "is_base": false
        }))
        .to_request();
    test::call_service(&app, req).await;

    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/currencies?active_only=true",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let items = json["items"].as_array().unwrap();
    for item in items {
        assert_eq!(item["is_active"], true);
    }
}

#[actix_web::test]
async fn test_get_currency_success() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(actix_web::http::Method::POST, "/api/currencies", &token)
        .set_json(json!({
            "code": "TRY",
            "name": "Turkish Lira",
            "symbol": "₺",
            "decimal_places": 2,
            "is_active": true,
            "is_base": true
        }))
        .to_request();
    test::call_service(&app, create_req).await;

    let get_req =
        auth_request(actix_web::http::Method::GET, "/api/currencies/TRY", &token).to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), StatusCode::OK);

    let body = to_bytes(get_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["code"], "TRY");
    assert_eq!(json["is_base"], true);
}

#[actix_web::test]
async fn test_get_currency_not_found() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req =
        auth_request(actix_web::http::Method::GET, "/api/currencies/ZZZ", &token).to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[actix_web::test]
async fn test_update_currency_success() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(actix_web::http::Method::POST, "/api/currencies", &token)
        .set_json(json!({
            "code": "JPY",
            "name": "Japanese Yen",
            "symbol": "¥",
            "decimal_places": 0,
            "is_active": true,
            "is_base": false
        }))
        .to_request();
    test::call_service(&app, create_req).await;

    let update_req = auth_request(actix_web::http::Method::PUT, "/api/currencies/JPY", &token)
        .set_json(json!({
            "name": "Updated Yen",
            "is_active": false
        }))
        .to_request();
    let update_resp = test::call_service(&app, update_req).await;
    assert_eq!(update_resp.status(), StatusCode::OK);

    let body = to_bytes(update_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["name"], "Updated Yen");
    assert_eq!(json["is_active"], false);
    assert_eq!(json["code"], "JPY");
}

// ============================================================================
// Soft Delete Tests
// ============================================================================

#[actix_web::test]
async fn test_soft_delete_and_restore_currency() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(actix_web::http::Method::POST, "/api/currencies", &token)
        .set_json(json!({
            "code": "CAD",
            "name": "Canadian Dollar",
            "symbol": "C$",
            "decimal_places": 2,
            "is_active": true,
            "is_base": false
        }))
        .to_request();
    test::call_service(&app, create_req).await;

    let del_req = auth_request(
        actix_web::http::Method::DELETE,
        "/api/currencies/CAD/soft",
        &token,
    )
    .to_request();
    let del_resp = test::call_service(&app, del_req).await;
    assert_eq!(del_resp.status(), StatusCode::NO_CONTENT);

    let get_req =
        auth_request(actix_web::http::Method::GET, "/api/currencies/CAD", &token).to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), StatusCode::NOT_FOUND);

    let restore_req = auth_request(
        actix_web::http::Method::POST,
        "/api/currencies/CAD/restore",
        &token,
    )
    .to_request();
    let restore_resp = test::call_service(&app, restore_req).await;
    assert_eq!(restore_resp.status(), StatusCode::NO_CONTENT);

    let get_req =
        auth_request(actix_web::http::Method::GET, "/api/currencies/CAD", &token).to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), StatusCode::OK);
}

#[actix_web::test]
async fn test_list_deleted_currencies() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(actix_web::http::Method::POST, "/api/currencies", &token)
        .set_json(json!({
            "code": "AUD",
            "name": "Australian Dollar",
            "symbol": "A$",
            "decimal_places": 2,
            "is_active": true,
            "is_base": false
        }))
        .to_request();
    test::call_service(&app, create_req).await;

    let del_req = auth_request(
        actix_web::http::Method::DELETE,
        "/api/currencies/AUD/soft",
        &token,
    )
    .to_request();
    test::call_service(&app, del_req).await;

    let list_req = auth_request(
        actix_web::http::Method::GET,
        "/api/currencies/deleted",
        &token,
    )
    .to_request();
    let list_resp = test::call_service(&app, list_req).await;
    assert_eq!(list_resp.status(), StatusCode::OK);

    let body = to_bytes(list_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let items = json.as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["code"], "AUD");
}

#[actix_web::test]
async fn test_destroy_currency_permanently() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let create_req = auth_request(actix_web::http::Method::POST, "/api/currencies", &token)
        .set_json(json!({
            "code": "CHF",
            "name": "Swiss Franc",
            "symbol": "Fr",
            "decimal_places": 2,
            "is_active": true,
            "is_base": false
        }))
        .to_request();
    test::call_service(&app, create_req).await;

    let del_req = auth_request(
        actix_web::http::Method::DELETE,
        "/api/currencies/CHF/soft",
        &token,
    )
    .to_request();
    test::call_service(&app, del_req).await;

    let destroy_req = auth_request(
        actix_web::http::Method::DELETE,
        "/api/currencies/CHF/destroy",
        &token,
    )
    .to_request();
    let destroy_resp = test::call_service(&app, destroy_req).await;
    assert_eq!(destroy_resp.status(), StatusCode::NO_CONTENT);

    let restore_req = auth_request(
        actix_web::http::Method::POST,
        "/api/currencies/CHF/restore",
        &token,
    )
    .to_request();
    let restore_resp = test::call_service(&app, restore_req).await;
    assert_eq!(restore_resp.status(), StatusCode::NOT_FOUND);
}

macro_rules! ensure_currency_exists {
    ($app:expr, $token:expr, $code:expr, $name:expr, $symbol:expr) => {{
        let req = auth_request(actix_web::http::Method::POST, "/api/currencies", $token)
            .set_json(json!({
                "code": $code,
                "name": $name,
                "symbol": $symbol,
                "decimal_places": 2,
                "is_active": true,
                "is_base": false
            }))
            .to_request();
        let resp = test::call_service($app, req).await;
        if resp.status() != StatusCode::CREATED && resp.status() != StatusCode::CONFLICT {
            let _body = to_bytes(resp.into_body()).await;
        }
    }};
}

// ============================================================================
// Exchange Rate Tests
// ============================================================================

#[actix_web::test]
async fn test_create_exchange_rate_success() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    ensure_currency_exists!(&app, &token, "USD", "US Dollar", "$");
    ensure_currency_exists!(&app, &token, "EUR", "Euro", "€");

    let req = auth_request(actix_web::http::Method::POST, "/api/exchange-rates", &token)
        .set_json(json!({
            "from_currency": "USD",
            "to_currency": "EUR",
            "rate": "0.85",
            "effective_date": "2024-01-01"
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["from_currency"], "USD");
    assert_eq!(json["to_currency"], "EUR");
    assert_eq!(json["rate"], "0.85");
    assert!(json["id"].is_number());
}

#[actix_web::test]
async fn test_list_exchange_rates_paginated() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    ensure_currency_exists!(&app, &token, "USD", "US Dollar", "$");

    for code in ["XAA", "XAB", "XAC"] {
        ensure_currency_exists!(&app, &token, code, &format!("Currency {}", code), "C");
        let req = auth_request(actix_web::http::Method::POST, "/api/exchange-rates", &token)
            .set_json(json!({
                "from_currency": "USD",
                "to_currency": code,
                "rate": "1.10",
                "effective_date": "2024-01-01"
            }))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::CREATED);
    }

    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/exchange-rates?page=1&per_page=2",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["items"].as_array().unwrap().len(), 2);
    assert_eq!(json["total"], 3);
}

#[actix_web::test]
async fn test_soft_delete_and_restore_exchange_rate() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    ensure_currency_exists!(&app, &token, "GBP", "British Pound", "£");
    ensure_currency_exists!(&app, &token, "USD", "US Dollar", "$");

    let create_req = auth_request(actix_web::http::Method::POST, "/api/exchange-rates", &token)
        .set_json(json!({
            "from_currency": "GBP",
            "to_currency": "USD",
            "rate": "1.25",
            "effective_date": "2024-01-01"
        }))
        .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    let del_req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/exchange-rates/{}/soft", id),
        &token,
    )
    .to_request();
    let del_resp = test::call_service(&app, del_req).await;
    assert_eq!(del_resp.status(), StatusCode::NO_CONTENT);

    let restore_req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/exchange-rates/{}/restore", id),
        &token,
    )
    .to_request();
    let restore_resp = test::call_service(&app, restore_req).await;
    assert_eq!(restore_resp.status(), StatusCode::NO_CONTENT);
}

#[actix_web::test]
async fn test_list_deleted_exchange_rates() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    ensure_currency_exists!(&app, &token, "EUR", "Euro", "€");
    ensure_currency_exists!(&app, &token, "USD", "US Dollar", "$");

    let create_req = auth_request(actix_web::http::Method::POST, "/api/exchange-rates", &token)
        .set_json(json!({
            "from_currency": "EUR",
            "to_currency": "USD",
            "rate": "1.10",
            "effective_date": "2024-01-01"
        }))
        .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    let del_req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/exchange-rates/{}/soft", id),
        &token,
    )
    .to_request();
    test::call_service(&app, del_req).await;

    let list_req = auth_request(
        actix_web::http::Method::GET,
        "/api/exchange-rates/deleted",
        &token,
    )
    .to_request();
    let list_resp = test::call_service(&app, list_req).await;
    assert_eq!(list_resp.status(), StatusCode::OK);

    let body = to_bytes(list_resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let items = json.as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["id"], id);
}

#[actix_web::test]
async fn test_convert_amount() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    ensure_currency_exists!(&app, &token, "USD", "US Dollar", "$");
    ensure_currency_exists!(&app, &token, "TRY", "Turkish Lira", "₺");

    let req = auth_request(actix_web::http::Method::POST, "/api/exchange-rates", &token)
        .set_json(json!({
            "from_currency": "USD",
            "to_currency": "TRY",
            "rate": "30.00",
            "effective_date": "2024-01-01"
        }))
        .to_request();
    test::call_service(&app, req).await;

    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/exchange-rates/convert?amount=100&from=USD&to=TRY&date=2024-01-01",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["from_currency"], "USD");
    assert_eq!(json["to_currency"], "TRY");
    assert_eq!(json["converted_amount"], "3000.00");
}

#[actix_web::test]
async fn test_get_effective_exchange_rate() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    ensure_currency_exists!(&app, &token, "EUR", "Euro", "€");
    ensure_currency_exists!(&app, &token, "TRY", "Turkish Lira", "₺");

    let req = auth_request(actix_web::http::Method::POST, "/api/exchange-rates", &token)
        .set_json(json!({
            "from_currency": "EUR",
            "to_currency": "TRY",
            "rate": "35.00",
            "effective_date": "2024-01-01"
        }))
        .to_request();
    test::call_service(&app, req).await;

    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/exchange-rates/effective?from=EUR&to=TRY&date=2024-01-01",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["from_currency"], "EUR");
    assert_eq!(json["to_currency"], "TRY");
    assert_eq!(json["rate"], "35.00");
}

// ============================================================================
// Unauthorized / Not Found
// ============================================================================

#[actix_web::test]
async fn test_currency_unauthorized_without_token() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;

    let req = test::TestRequest::post()
        .uri("/api/currencies")
        .set_json(json!({
            "code": "XYZ",
            "name": "No Auth",
            "symbol": "X",
            "is_active": true,
            "is_base": false
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn test_currency_not_found() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let req =
        auth_request(actix_web::http::Method::GET, "/api/currencies/ZZZ", &token).to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}
