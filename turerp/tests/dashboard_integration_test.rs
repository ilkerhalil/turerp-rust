//! Dashboard API Integration Tests

use actix_web::{body::to_bytes, http::StatusCode, test};
use serde_json::json;

use crate::common::*;

#[actix_web::test]
async fn test_get_all_kpis() {
    let app_state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&app_state)).await;
    let (token, _) = register_admin(&app_state, 1).await;

    let req = test::TestRequest::get()
        .uri("/api/v1/dashboard/kpis")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    // KpiResponse is an object with fields like revenue, profit, etc.
    assert!(json.is_object());
    assert!(json.get("revenue").is_some() || json.get("stock_value").is_some());
}

#[actix_web::test]
async fn test_get_single_kpi() {
    let app_state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&app_state)).await;
    let (token, _) = register_admin(&app_state, 1).await;

    let req = test::TestRequest::get()
        .uri("/api/v1/dashboard/kpis/revenue")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json["name"].is_string() || json["value"].is_number());
}

#[actix_web::test]
async fn test_get_sales_chart() {
    let app_state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&app_state)).await;
    let (token, _) = register_admin(&app_state, 1).await;

    let req = test::TestRequest::get()
        .uri("/api/v1/dashboard/charts/sales")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json.is_object());
}

#[actix_web::test]
async fn test_get_revenue_by_category_chart() {
    let app_state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&app_state)).await;
    let (token, _) = register_admin(&app_state, 1).await;

    let req = test::TestRequest::get()
        .uri("/api/v1/dashboard/charts/revenue-by-category")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

#[actix_web::test]
async fn test_get_top_products_chart() {
    let app_state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&app_state)).await;
    let (token, _) = register_admin(&app_state, 1).await;

    let req = test::TestRequest::get()
        .uri("/api/v1/dashboard/charts/top-products")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
}

#[actix_web::test]
async fn test_create_and_list_widgets() {
    let app_state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&app_state)).await;
    let (token, _) = register_admin(&app_state, 1).await;

    let create_req = test::TestRequest::post()
        .uri("/api/v1/dashboard/widgets")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "title": "Sales Widget",
            "widget_type": "BarChart",
            "position": {"x": 0, "y": 0, "w": 6, "h": 4}
        }))
        .to_request();
    let resp = test::call_service(&app, create_req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let list_req = test::TestRequest::get()
        .uri("/api/v1/dashboard/widgets")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, list_req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json.is_array());
}

#[actix_web::test]
async fn test_delete_widget() {
    let app_state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&app_state)).await;
    let (token, _) = register_admin(&app_state, 1).await;

    let create_req = test::TestRequest::post()
        .uri("/api/v1/dashboard/widgets")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .set_json(json!({
            "title": "Temp Widget",
            "widget_type": "Kpi",
            "position": {"x": 0, "y": 0, "w": 4, "h": 2}
        }))
        .to_request();
    let resp = test::call_service(&app, create_req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let id = create_json["id"].as_i64().unwrap();

    let delete_req = test::TestRequest::delete()
        .uri(&format!("/api/v1/dashboard/widgets/{}", id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();
    let resp = test::call_service(&app, delete_req).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);
}

#[actix_web::test]
async fn test_dashboard_requires_auth() {
    let app_state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&app_state)).await;

    let req = test::TestRequest::get()
        .uri("/api/v1/dashboard/kpis")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn test_dashboard_tenant_isolation() {
    let app_state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&app_state)).await;
    let (token1, _) = register_admin(&app_state, 1).await;
    let (token2, _) = register_admin(&app_state, 2).await;

    // Create widget in tenant 1
    let create_req = test::TestRequest::post()
        .uri("/api/v1/dashboard/widgets")
        .insert_header(("Authorization", format!("Bearer {}", token1)))
        .set_json(json!({
            "title": "Tenant1 Widget",
            "widget_type": "Kpi",
            "position": {"x": 0, "y": 0, "w": 4, "h": 2}
        }))
        .to_request();
    let resp = test::call_service(&app, create_req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let create_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let widget_id = create_json["id"].as_i64().unwrap();

    // Tenant 2 should not see the widget in their list
    let req = test::TestRequest::get()
        .uri("/api/v1/dashboard/widgets")
        .insert_header(("Authorization", format!("Bearer {}", token2)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let widgets = json.as_array().unwrap();
    for widget in widgets {
        assert_ne!(widget["tenant_id"], 1);
        assert_ne!(widget["id"], widget_id);
    }
}
