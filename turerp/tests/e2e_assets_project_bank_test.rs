//! End-to-End workflow tests for Assets (18), Project (14), Bank (19),
//! and Archive (14) endpoints.
//!
//! Run with: `cargo test --test integration e2e_assets_project_bank`

use actix_web::{body::to_bytes, http::StatusCode, test, web, App};
use serde_json::{json, Value};

use crate::common::*;
use turerp::api::{auth_configure, v1_archive_configure};

/// Build a test app that includes archive routes (not registered in the
/// default `build_test_app`).
fn build_test_app_with_archive(
    state: &turerp::app::AppState,
) -> App<
    impl actix_web::dev::ServiceFactory<
        actix_web::dev::ServiceRequest,
        Config = (),
        Response = actix_web::dev::ServiceResponse<
            actix_web::body::EitherBody<actix_web::body::BoxBody>,
        >,
        Error = actix_web::Error,
        InitError = (),
    >,
> {
    let jwt = create_test_jwt_service();
    App::new()
        .wrap(turerp::middleware::JwtAuthMiddleware::new(jwt))
        .app_data(web::Data::new(state.clone()))
        .app_data(state.auth.auth_service.clone())
        .app_data(state.auth.user_service.clone())
        .app_data(state.auth.jwt_service.clone())
        .app_data(state.analytics.archive_service.clone())
        .service(
            web::scope("/api")
                .configure(auth_configure)
                .configure(v1_archive_configure),
        )
}

// ============================================================================
// ASSETS — 18 endpoints
// ============================================================================

#[actix_web::test]
async fn e2e_assets_full_workflow() {
    let app_state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&app_state)).await;
    let (token, _user_id) = register_admin(&app_state, 1).await;

    // Create
    let req = auth_request(actix_web::http::Method::POST, "/api/v1/assets", &token)
        .set_json(json!({
            "tenant_id": 1,
            "name": format!("E2E Asset {}", uuid::Uuid::new_v4()),
            "acquisition_date": "2024-01-15T00:00:00Z",
            "acquisition_cost": "10000.00",
            "salvage_value": "1000.00",
            "useful_life_years": 5,
            "depreciation_method": "straightline",
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED, "create asset");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let asset_id = json["id"].as_i64().unwrap();
    assert_eq!(json["status"], "active");

    // List
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/assets?page=1&per_page=10",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "list assets");

    // Get by ID
    let req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/assets/{}", asset_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "get asset");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["id"], asset_id);

    // Get by status
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/assets/status/active",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "get assets by status");

    // Update asset
    let req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/assets/{}", asset_id),
        &token,
    )
    .set_json(json!({
        "name": "E2E Asset Renamed",
        "location": "Warehouse B",
    }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "update asset");

    // Update status to InUse
    let req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/assets/{}/status", asset_id),
        &token,
    )
    .set_json(json!({ "status": "inuse" }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "update asset status");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "inuse");

    // Calculate depreciation
    let req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/assets/{}/depreciation", asset_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "calculate depreciation");

    // Record depreciation
    let req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/assets/{}/depreciation/record", asset_id),
        &token,
    )
    .set_json(json!({ "amount": "500.00" }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "record depreciation");

    // Start maintenance
    let req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/assets/{}/maintenance/start", asset_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "start maintenance");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "undermaintenance");

    // End maintenance
    let req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/assets/{}/maintenance/end", asset_id),
        &token,
    )
    .set_json(json!({ "new_status": "active" }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "end maintenance");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "active");

    // Dispose
    let req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/assets/{}/dispose", asset_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "dispose asset");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "disposed");

    // Soft delete
    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/assets/{}", asset_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "soft delete asset");

    // Restore
    let req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/assets/{}/restore", asset_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "restore asset");

    // Soft delete again before listing deleted & destroying
    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/assets/{}", asset_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "soft delete asset (2nd)");

    // List deleted
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/assets/deleted",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "list deleted assets");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let deleted_arr = json.as_array().expect("deleted assets should be an array");
    assert!(
        deleted_arr.iter().any(|a| a["id"] == asset_id),
        "soft-deleted asset should appear in deleted list"
    );

    // Destroy (hard delete)
    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/assets/{}/destroy", asset_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT, "destroy asset");

    // Verify destroyed asset is gone
    let req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/assets/{}", asset_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::NOT_FOUND,
        "destroyed asset not found"
    );
}

#[actix_web::test]
async fn e2e_assets_maintenance_records() {
    let app_state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&app_state)).await;
    let (token, _user_id) = register_admin(&app_state, 1).await;

    // Create asset
    let req = auth_request(actix_web::http::Method::POST, "/api/v1/assets", &token)
        .set_json(json!({
            "tenant_id": 1,
            "name": format!("Maint Asset {}", uuid::Uuid::new_v4()),
            "acquisition_date": "2024-02-01T00:00:00Z",
            "acquisition_cost": "5000.00",
            "salvage_value": "500.00",
            "useful_life_years": 3,
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let asset_id = json["id"].as_i64().unwrap();

    // Create maintenance record
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/assets/maintenance-records",
        &token,
    )
    .set_json(json!({
        "asset_id": asset_id,
        "maintenance_date": "2024-03-01T00:00:00Z",
        "maintenance_type": "Preventive",
        "description": "Quarterly service",
        "cost": "250.00",
        "performed_by": "Technician A",
        "next_maintenance_date": "2024-06-01T00:00:00Z",
    }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::CREATED,
        "create maintenance record"
    );
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["asset_id"], asset_id);
    assert_eq!(json["maintenance_type"], "Preventive");

    // Get maintenance records
    let req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/assets/{}/maintenance-records", asset_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "get maintenance records");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let arr = json
        .as_array()
        .expect("maintenance records should be an array");
    assert!(
        !arr.is_empty(),
        "should have at least one maintenance record"
    );
    assert_eq!(arr[0]["asset_id"], asset_id);
}

#[actix_web::test]
async fn e2e_assets_write_off() {
    let app_state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&app_state)).await;
    let (token, _user_id) = register_admin(&app_state, 1).await;

    // Create asset
    let req = auth_request(actix_web::http::Method::POST, "/api/v1/assets", &token)
        .set_json(json!({
            "tenant_id": 1,
            "name": format!("WO Asset {}", uuid::Uuid::new_v4()),
            "acquisition_date": "2024-01-01T00:00:00Z",
            "acquisition_cost": "2000.00",
            "salvage_value": "0.00",
            "useful_life_years": 2,
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let asset_id = json["id"].as_i64().unwrap();

    // Write off
    let req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/assets/{}/write-off", asset_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "write off asset");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "writtenoff");
}

// ============================================================================
// PROJECT — 14 endpoints
// ============================================================================

#[actix_web::test]
async fn e2e_projects_full_workflow() {
    let app_state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&app_state)).await;
    let (token, _user_id) = register_admin(&app_state, 1).await;

    // Create project
    let req = auth_request(actix_web::http::Method::POST, "/api/v1/projects", &token)
        .set_json(json!({
            "tenant_id": 1,
            "name": format!("E2E Project {}", uuid::Uuid::new_v4()),
            "description": "End-to-end project workflow",
            "budget": "50000.00",
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED, "create project");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let project_id = json["id"].as_i64().unwrap();
    assert_eq!(json["status"], "Planning");

    // List
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/projects?page=1&per_page=10",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "list projects");

    // Get by ID
    let req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/projects/{}", project_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "get project");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["id"], project_id);

    // Update status to Active
    let req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/projects/{}/status", project_id),
        &token,
    )
    .set_json(json!({ "status": "Active" }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "update project status");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "Active");

    // Add WBS item
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/projects/wbs",
        &token,
    )
    .set_json(json!({
        "project_id": project_id,
        "parent_id": null,
        "name": "Phase 1",
        "code": format!("WBS-{}", uuid::Uuid::new_v4()),
        "planned_hours": "100.00",
    }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED, "create wbs item");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let wbs_id = json["id"].as_i64().unwrap();
    assert_eq!(json["project_id"], project_id);

    // Get WBS items
    let req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/projects/{}/wbs", project_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "get wbs");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let arr = json.as_array().expect("wbs should be an array");
    assert!(!arr.is_empty());

    // Update WBS progress
    let req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/projects/wbs/{}/progress", wbs_id),
        &token,
    )
    .set_json(json!({ "progress": "50.00", "hours": "40.00" }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "update wbs progress");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["progress"], "50.00");

    // Add project cost
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/projects/costs",
        &token,
    )
    .set_json(json!({
        "project_id": project_id,
        "wbs_item_id": wbs_id,
        "cost_type": "Labor",
        "amount": "5000.00",
        "description": "Developer hours",
        "incurred_at": "2024-04-01T00:00:00Z",
    }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED, "create project cost");

    // Get project costs
    let req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/projects/{}/costs", project_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "get project costs");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let arr = json.as_array().expect("costs should be an array");
    assert!(!arr.is_empty());

    // Get profitability
    let req = auth_request(
        actix_web::http::Method::GET,
        &format!(
            "/api/v1/projects/{}/profitability?revenue=80000.00",
            project_id
        ),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "get profitability");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["project_id"], project_id);
    assert_eq!(json["revenue"], "80000.00");

    // Soft delete
    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/projects/{}", project_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "soft delete project");

    // Restore
    let req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/projects/{}/restore", project_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "restore project");

    // Soft delete again before listing deleted & destroying
    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/projects/{}", project_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "soft delete project (2nd)");

    // List deleted
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/projects/deleted",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "list deleted projects");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let arr = json
        .as_array()
        .expect("deleted projects should be an array");
    assert!(
        arr.iter().any(|p| p["id"] == project_id),
        "soft-deleted project should appear in deleted list"
    );

    // Destroy
    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/projects/{}/destroy", project_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT, "destroy project");

    // Verify destroyed
    let req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/projects/{}", project_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::NOT_FOUND,
        "destroyed project not found"
    );
}

// ============================================================================
// BANK — 19 endpoints
// ============================================================================

#[actix_web::test]
async fn e2e_bank_accounts_workflow() {
    let app_state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&app_state)).await;
    let (token, _user_id) = register_admin(&app_state, 1).await;

    // Create account
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/bank/accounts",
        &token,
    )
    .set_json(json!({
        "bank_code": "garanti",
        "account_number": format!("E2E-{}", uuid::Uuid::new_v4()),
        "account_name": "E2E Main Account",
        "currency": "TRY",
        "iban": "TR000123456789012345678901",
        "branch_code": "001",
        "is_active": true,
        "tenant_id": 1,
    }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED, "create bank account");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let account_id = json["id"].as_i64().unwrap();
    assert_eq!(json["account_name"], "E2E Main Account");

    // List
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/bank/accounts",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "list bank accounts");

    // Get by ID
    let req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/bank/accounts/{}", account_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "get bank account");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["id"], account_id);

    // Update account
    let req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/bank/accounts/{}", account_id),
        &token,
    )
    .set_json(json!({
        "account_name": "E2E Renamed Account",
        "is_active": false,
    }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "update bank account");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["account_name"], "E2E Renamed Account");

    // Add statement (import transactions)
    let mt940_data = ":61:230101C500,00NTRF//REF002\n:86:Payment received\n";
    let req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/bank/accounts/{}/statements", account_id),
        &token,
    )
    .set_json(json!({
        "format": "mt940",
        "data": mt940_data,
    }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED, "import statement");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert!(json["transactions_imported"].as_i64().unwrap_or(0) >= 1);

    // Get transactions
    let req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/bank/accounts/{}/transactions", account_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "get transactions");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let txns = json.as_array().expect("transactions should be an array");
    assert!(!txns.is_empty());

    // Get unmatched transactions
    let req = auth_request(
        actix_web::http::Method::GET,
        &format!(
            "/api/v1/bank/accounts/{}/transactions/unmatched",
            account_id
        ),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "get unmatched transactions");

    // Soft delete
    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/bank/accounts/{}", account_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "soft delete bank account");

    // Restore
    let req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/bank/accounts/{}/restore", account_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "restore bank account");

    // Soft delete again before destroy
    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/bank/accounts/{}", account_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "soft delete bank account (2nd)"
    );

    // Destroy
    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/bank/accounts/{}/destroy", account_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::NO_CONTENT,
        "destroy bank account"
    );

    // Verify destroyed
    let req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/bank/accounts/{}", account_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::NOT_FOUND,
        "destroyed account not found"
    );
}

#[actix_web::test]
async fn e2e_bank_rules_workflow() {
    let app_state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&app_state)).await;
    let (token, _user_id) = register_admin(&app_state, 1).await;

    // Create rule
    let req = auth_request(actix_web::http::Method::POST, "/api/v1/bank/rules", &token)
        .set_json(json!({
            "rule_name": format!("E2E Rule {}", uuid::Uuid::new_v4()),
            "match_field": "description",
            "match_pattern": "Payment",
            "auto_match": true,
            "is_active": true,
            "tenant_id": 1,
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED, "create bank rule");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let rule_id = json["id"].as_i64().unwrap();
    assert_eq!(json["match_field"], "description");

    // List
    let req = auth_request(actix_web::http::Method::GET, "/api/v1/bank/rules", &token).to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "list bank rules");

    // Get by ID
    let req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/bank/rules/{}", rule_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "get bank rule");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["id"], rule_id);

    // Update rule
    let req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/bank/rules/{}", rule_id),
        &token,
    )
    .set_json(json!({
        "rule_name": "E2E Rule Updated",
        "auto_match": false,
    }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "update bank rule");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["rule_name"], "E2E Rule Updated");

    // Delete rule
    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/bank/rules/{}", rule_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "delete bank rule");

    // Verify deleted
    let req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/bank/rules/{}", rule_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::NOT_FOUND,
        "deleted rule not found"
    );
}

#[actix_web::test]
async fn e2e_bank_reconciliation() {
    let app_state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&app_state)).await;
    let (token, _user_id) = register_admin(&app_state, 1).await;

    // GET reconciliation report
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/bank/reconciliation",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "get reconciliation report");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert!(json["total_transactions"].is_number());

    // POST reconcile (auto-reconciliation)
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/bank/reconcile",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "auto reconcile");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert!(json["total_transactions"].is_number());
}

// ============================================================================
// ARCHIVE — 14 endpoints
// ============================================================================

#[actix_web::test]
async fn e2e_archive_policies_workflow() {
    let app_state = create_test_app_state().await;
    let app = test::init_service(build_test_app_with_archive(&app_state)).await;
    let (token, _user_id) = register_admin(&app_state, 1).await;

    // Create policy
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/archive/policies",
        &token,
    )
    .set_json(json!({
        "name": format!("E2E Policy {}", uuid::Uuid::new_v4()),
        "table_name": "invoices",
        "age_days": 365,
        "is_active": true,
    }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED, "create archive policy");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let policy_id = json["id"].as_i64().unwrap();
    assert_eq!(json["table_name"], "invoices");

    // List
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/archive/policies?page=1&per_page=10",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "list archive policies");

    // Get by ID
    let req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/archive/policies/{}", policy_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "get archive policy");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["id"], policy_id);

    // List active policies
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/archive/policies/active",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "list active policies");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let arr = json.as_array().expect("active policies should be an array");
    assert!(arr.iter().any(|p| p["id"] == policy_id));

    // Update policy
    let req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/archive/policies/{}", policy_id),
        &token,
    )
    .set_json(json!({
        "name": "E2E Policy Updated",
        "age_days": 730,
        "is_active": false,
    }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "update archive policy");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["name"], "E2E Policy Updated");
    assert_eq!(json["age_days"], 730);
    assert_eq!(json["is_active"], false);

    // Delete policy
    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/archive/policies/{}", policy_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::NO_CONTENT,
        "delete archive policy"
    );

    // Verify deleted
    let req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/archive/policies/{}", policy_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::NOT_FOUND,
        "deleted policy not found"
    );
}

#[actix_web::test]
async fn e2e_archive_jobs_workflow() {
    let app_state = create_test_app_state().await;
    let app = test::init_service(build_test_app_with_archive(&app_state)).await;
    let (token, _user_id) = register_admin(&app_state, 1).await;

    // Create a policy first (job requires a valid policy_id)
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/archive/policies",
        &token,
    )
    .set_json(json!({
        "name": format!("E2E Job Policy {}", uuid::Uuid::new_v4()),
        "table_name": "invoices",
        "age_days": 180,
        "is_active": true,
    }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED, "create policy for job");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let policy_id = json["id"].as_i64().unwrap();

    // Create job
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/archive/jobs",
        &token,
    )
    .set_json(json!({ "policy_id": policy_id }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED, "create archive job");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let job_id = json["id"].as_i64().unwrap();
    assert_eq!(json["policy_id"], policy_id);

    // List jobs
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/archive/jobs?page=1&per_page=10",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "list archive jobs");

    // Get job by ID
    let req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/archive/jobs/{}", job_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "get archive job");
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["id"], job_id);

    // Delete job
    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/archive/jobs/{}", job_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT, "delete archive job");

    // Verify deleted
    let req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/archive/jobs/{}", job_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::NOT_FOUND,
        "deleted job not found"
    );
}

#[actix_web::test]
async fn e2e_archive_records_restore() {
    let app_state = create_test_app_state().await;
    let app = test::init_service(build_test_app_with_archive(&app_state)).await;
    let (token, _user_id) = register_admin(&app_state, 1).await;

    // List records (initially empty)
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/archive/records?page=1&per_page=10",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "list archive records");

    // Restore with empty list → 400 BadRequest
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/archive/records/restore",
        &token,
    )
    .set_json(json!({ "record_ids": [] }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::BAD_REQUEST,
        "empty record_ids should be rejected"
    );

    // Restore with a non-existent record id → 200 with failed entry
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/archive/records/restore",
        &token,
    )
    .set_json(json!({ "record_ids": [999999] }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "restore with non-existent id should still return 200"
    );
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["restored"], 0, "no records should be restored");
    let failed = json["failed"]
        .as_array()
        .expect("failed should be an array");
    assert_eq!(failed.len(), 1, "one failed entry for the non-existent id");
    assert_eq!(failed[0]["id"], 999999);
}
