//! End-to-End Manufacturing Integration Tests
//!
//! Exercises all 46 Manufacturing endpoints (BOMs, routings, work orders,
//! inspections, NCRs, material requirements) through the full HTTP stack
//! against the in-memory backend. Each test function walks a complete
//! lifecycle: create → read → update → soft-delete → restore → deleted-list →
//! destroy, plus the sub-resource attachment endpoints (BOM lines, routing
//! operations, work-order materials/operations).
//!
//! Run with: cargo test --test integration e2e_manufacturing

use actix_web::{body::to_bytes, http::StatusCode, test};
use serde_json::{json, Value};

use crate::common::*;

// ============================================================================
// Helpers
// ============================================================================

/// Extract the `id` field from a JSON response body as i64.
fn extract_id(json: &Value) -> i64 {
    json["id"]
        .as_i64()
        .or_else(|| json["id"].as_u64().map(|u| u as i64))
        .expect("response has numeric id")
}

// ============================================================================
// BOMs Workflow
// ============================================================================

/// Full BOM lifecycle: create product → create BOM → add BOM line → get BOM →
/// get BOMs by product → get BOM lines → soft delete → restore → deleted list →
/// destroy.
#[actix_web::test]
async fn e2e_manufacturing_boms_workflow() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    // Create a parent product and a component product.
    let parent_product_id = seed_product!(&app, &token, 1);
    let component_product_id = seed_product!(&app, &token, 1);

    // POST /api/v1/manufacturing/boms — create a primary BOM for the product.
    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/manufacturing/boms",
        &token,
    )
    .set_json(json!({
        "tenant_id": 1,
        "product_id": parent_product_id,
        "version": "1.0",
        "is_active": true,
        "is_primary": true,
        "valid_from": null,
        "valid_to": null
    }))
    .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    assert_eq!(create_resp.status(), StatusCode::CREATED, "create bom");
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let bom_json: Value = serde_json::from_slice(&body).unwrap();
    let bom_id = extract_id(&bom_json);
    assert_eq!(bom_json["product_id"], parent_product_id);
    assert_eq!(bom_json["version"], "1.0");
    assert_eq!(bom_json["is_primary"], true);

    // POST /api/v1/manufacturing/boms/lines — attach a component line.
    let line_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/manufacturing/boms/lines",
        &token,
    )
    .set_json(json!({
        "tenant_id": 1,
        "bom_id": bom_id,
        "component_product_id": component_product_id,
        "quantity": "5.00",
        "unit_id": null,
        "scrap_percentage": "2.00",
        "is_optional": false,
        "notes": "E2E component line"
    }))
    .to_request();
    let line_resp = test::call_service(&app, line_req).await;
    assert_eq!(line_resp.status(), StatusCode::CREATED, "add bom line");
    let body = to_bytes(line_resp.into_body()).await.unwrap();
    let line_json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(line_json["bom_id"], bom_id);
    assert_eq!(line_json["component_product_id"], component_product_id);

    // GET /api/v1/manufacturing/boms/{id} — fetch the BOM by id.
    let get_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/manufacturing/boms/{}", bom_id),
        &token,
    )
    .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), StatusCode::OK, "get bom");
    let body = to_bytes(get_resp.into_body()).await.unwrap();
    let get_json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(get_json["id"], bom_id);

    // GET /api/v1/manufacturing/boms/product/{product_id} — list BOMs for product.
    let by_product_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/manufacturing/boms/product/{}", parent_product_id),
        &token,
    )
    .to_request();
    let by_product_resp = test::call_service(&app, by_product_req).await;
    assert_eq!(
        by_product_resp.status(),
        StatusCode::OK,
        "get boms by product"
    );
    let body = to_bytes(by_product_resp.into_body()).await.unwrap();
    let by_product_json: Value = serde_json::from_slice(&body).unwrap();
    let boms = by_product_json
        .as_array()
        .expect("boms by product is array");
    assert_eq!(boms.len(), 1);
    assert_eq!(boms[0]["id"], bom_id);

    // GET /api/v1/manufacturing/boms/{bom_id}/lines — fetch the BOM lines.
    let lines_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/manufacturing/boms/{}/lines", bom_id),
        &token,
    )
    .to_request();
    let lines_resp = test::call_service(&app, lines_req).await;
    assert_eq!(lines_resp.status(), StatusCode::OK, "get bom lines");
    let body = to_bytes(lines_resp.into_body()).await.unwrap();
    let lines_json: Value = serde_json::from_slice(&body).unwrap();
    let lines = lines_json.as_array().expect("bom lines is array");
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0]["component_product_id"], component_product_id);

    // DELETE /api/v1/manufacturing/boms/{id} — soft delete.
    let del_req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/manufacturing/boms/{}", bom_id),
        &token,
    )
    .to_request();
    let del_resp = test::call_service(&app, del_req).await;
    assert_eq!(del_resp.status(), StatusCode::OK, "soft delete bom");

    // Verify the BOM is now hidden from the normal get endpoint.
    let get_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/manufacturing/boms/{}", bom_id),
        &token,
    )
    .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(
        get_resp.status(),
        StatusCode::NOT_FOUND,
        "deleted bom is 404"
    );

    // GET /api/v1/manufacturing/boms/deleted — list soft-deleted BOMs.
    let deleted_req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/manufacturing/boms/deleted",
        &token,
    )
    .to_request();
    let deleted_resp = test::call_service(&app, deleted_req).await;
    assert_eq!(deleted_resp.status(), StatusCode::OK, "list deleted boms");
    let body = to_bytes(deleted_resp.into_body()).await.unwrap();
    let deleted_json: Value = serde_json::from_slice(&body).unwrap();
    let deleted = deleted_json.as_array().expect("deleted boms is array");
    assert!(
        deleted.iter().any(|b| b["id"] == bom_id),
        "deleted list has bom"
    );

    // PUT /api/v1/manufacturing/boms/{id}/restore — restore the BOM.
    let restore_req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/manufacturing/boms/{}/restore", bom_id),
        &token,
    )
    .to_request();
    let restore_resp = test::call_service(&app, restore_req).await;
    assert_eq!(restore_resp.status(), StatusCode::OK, "restore bom");
    let body = to_bytes(restore_resp.into_body()).await.unwrap();
    let restore_json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(restore_json["id"], bom_id);

    // Verify the BOM is visible again.
    let get_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/manufacturing/boms/{}", bom_id),
        &token,
    )
    .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), StatusCode::OK, "restored bom is visible");

    // Soft-delete again before permanent destroy.
    let del_req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/manufacturing/boms/{}", bom_id),
        &token,
    )
    .to_request();
    test::call_service(&app, del_req).await;

    // DELETE /api/v1/manufacturing/boms/{id}/destroy — permanently destroy.
    let destroy_req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/manufacturing/boms/{}/destroy", bom_id),
        &token,
    )
    .to_request();
    let destroy_resp = test::call_service(&app, destroy_req).await;
    assert_eq!(destroy_resp.status(), StatusCode::NO_CONTENT, "destroy bom");

    // Restore should now 404 (record gone).
    let restore_req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/manufacturing/boms/{}/restore", bom_id),
        &token,
    )
    .to_request();
    let restore_resp = test::call_service(&app, restore_req).await;
    assert_eq!(
        restore_resp.status(),
        StatusCode::NOT_FOUND,
        "destroyed bom not restorable"
    );
}

// ============================================================================
// Routings Workflow
// ============================================================================

/// Full routing lifecycle: create product → create routing → add routing
/// operation → get routing → get routings by product → soft delete → restore →
/// deleted list → destroy.
#[actix_web::test]
async fn e2e_manufacturing_routings_workflow() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;
    let product_id = seed_product!(&app, &token, 1);

    // POST /api/v1/manufacturing/routings — create a primary routing.
    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/manufacturing/routings",
        &token,
    )
    .set_json(json!({
        "tenant_id": 1,
        "product_id": product_id,
        "version": "1.0",
        "is_active": true,
        "is_primary": true
    }))
    .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    assert_eq!(create_resp.status(), StatusCode::CREATED, "create routing");
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let routing_json: Value = serde_json::from_slice(&body).unwrap();
    let routing_id = extract_id(&routing_json);
    assert_eq!(routing_json["product_id"], product_id);
    assert_eq!(routing_json["version"], "1.0");
    assert_eq!(routing_json["is_primary"], true);

    // POST /api/v1/manufacturing/routings/operations — add an operation.
    let op_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/manufacturing/routings/operations",
        &token,
    )
    .set_json(json!({
        "tenant_id": 1,
        "routing_id": routing_id,
        "sequence": 10,
        "operation_name": "Cut",
        "work_center_id": null,
        "setup_hours": "0.50",
        "run_hours": "1.50",
        "description": "Cutting operation"
    }))
    .to_request();
    let op_resp = test::call_service(&app, op_req).await;
    assert_eq!(
        op_resp.status(),
        StatusCode::CREATED,
        "add routing operation"
    );
    let body = to_bytes(op_resp.into_body()).await.unwrap();
    let op_json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(op_json["routing_id"], routing_id);
    assert_eq!(op_json["operation_name"], "Cut");

    // GET /api/v1/manufacturing/routings/{id} — fetch the routing.
    let get_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/manufacturing/routings/{}", routing_id),
        &token,
    )
    .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), StatusCode::OK, "get routing");
    let body = to_bytes(get_resp.into_body()).await.unwrap();
    let get_json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(get_json["id"], routing_id);

    // GET /api/v1/manufacturing/routings/product/{product_id} — list by product.
    let by_product_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/manufacturing/routings/product/{}", product_id),
        &token,
    )
    .to_request();
    let by_product_resp = test::call_service(&app, by_product_req).await;
    assert_eq!(
        by_product_resp.status(),
        StatusCode::OK,
        "get routings by product"
    );
    let body = to_bytes(by_product_resp.into_body()).await.unwrap();
    let by_product_json: Value = serde_json::from_slice(&body).unwrap();
    let routings = by_product_json
        .as_array()
        .expect("routings by product is array");
    assert_eq!(routings.len(), 1);
    assert_eq!(routings[0]["id"], routing_id);

    // DELETE /api/v1/manufacturing/routings/{id} — soft delete.
    let del_req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/manufacturing/routings/{}", routing_id),
        &token,
    )
    .to_request();
    let del_resp = test::call_service(&app, del_req).await;
    assert_eq!(del_resp.status(), StatusCode::OK, "soft delete routing");

    // Verify it's hidden.
    let get_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/manufacturing/routings/{}", routing_id),
        &token,
    )
    .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(
        get_resp.status(),
        StatusCode::NOT_FOUND,
        "deleted routing is 404"
    );

    // GET /api/v1/manufacturing/routings/deleted — list soft-deleted routings.
    let deleted_req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/manufacturing/routings/deleted",
        &token,
    )
    .to_request();
    let deleted_resp = test::call_service(&app, deleted_req).await;
    assert_eq!(
        deleted_resp.status(),
        StatusCode::OK,
        "list deleted routings"
    );
    let body = to_bytes(deleted_resp.into_body()).await.unwrap();
    let deleted_json: Value = serde_json::from_slice(&body).unwrap();
    let deleted = deleted_json.as_array().expect("deleted routings is array");
    assert!(
        deleted.iter().any(|r| r["id"] == routing_id),
        "deleted list has routing"
    );

    // PUT /api/v1/manufacturing/routings/{id}/restore — restore.
    let restore_req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/manufacturing/routings/{}/restore", routing_id),
        &token,
    )
    .to_request();
    let restore_resp = test::call_service(&app, restore_req).await;
    assert_eq!(restore_resp.status(), StatusCode::OK, "restore routing");
    let body = to_bytes(restore_resp.into_body()).await.unwrap();
    let restore_json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(restore_json["id"], routing_id);

    // Verify visible again.
    let get_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/manufacturing/routings/{}", routing_id),
        &token,
    )
    .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(
        get_resp.status(),
        StatusCode::OK,
        "restored routing is visible"
    );

    // Soft-delete again before destroy.
    let del_req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/manufacturing/routings/{}", routing_id),
        &token,
    )
    .to_request();
    test::call_service(&app, del_req).await;

    // DELETE /api/v1/manufacturing/routings/{id}/destroy — permanently destroy.
    let destroy_req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/manufacturing/routings/{}/destroy", routing_id),
        &token,
    )
    .to_request();
    let destroy_resp = test::call_service(&app, destroy_req).await;
    assert_eq!(
        destroy_resp.status(),
        StatusCode::NO_CONTENT,
        "destroy routing"
    );

    let restore_req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/manufacturing/routings/{}/restore", routing_id),
        &token,
    )
    .to_request();
    let restore_resp = test::call_service(&app, restore_req).await;
    assert_eq!(
        restore_resp.status(),
        StatusCode::NOT_FOUND,
        "destroyed routing not restorable"
    );
}

// ============================================================================
// Work Orders Workflow
// ============================================================================

/// Full work-order lifecycle: create product → create BOM → create work order
/// (referencing the BOM) → add materials → add operations → get work order →
/// list work orders → get materials → get operations → update status → soft
/// delete → restore → deleted list → destroy.
#[actix_web::test]
async fn e2e_manufacturing_work_orders_workflow() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let product_id = seed_product!(&app, &token, 1);
    let material_product_id = seed_product!(&app, &token, 1);

    // Create a BOM to reference from the work order.
    let bom_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/manufacturing/boms",
        &token,
    )
    .set_json(json!({
        "tenant_id": 1,
        "product_id": product_id,
        "version": "1.0",
        "is_active": true,
        "is_primary": true,
        "valid_from": null,
        "valid_to": null
    }))
    .to_request();
    let bom_resp = test::call_service(&app, bom_req).await;
    assert_eq!(
        bom_resp.status(),
        StatusCode::CREATED,
        "create bom for work order"
    );
    let body = to_bytes(bom_resp.into_body()).await.unwrap();
    let bom_json: Value = serde_json::from_slice(&body).unwrap();
    let bom_id = extract_id(&bom_json);

    // POST /api/v1/manufacturing/work-orders — create a work order.
    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/manufacturing/work-orders",
        &token,
    )
    .set_json(json!({
        "tenant_id": 1,
        "name": format!("WO-E2E-{}", uuid::Uuid::new_v4()),
        "product_id": product_id,
        "quantity": "100.00",
        "bom_id": bom_id,
        "routing_id": null,
        "priority": "Normal",
        "planned_start": null,
        "planned_end": null
    }))
    .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    assert_eq!(
        create_resp.status(),
        StatusCode::CREATED,
        "create work order"
    );
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let wo_json: Value = serde_json::from_slice(&body).unwrap();
    let wo_id = extract_id(&wo_json);
    assert_eq!(wo_json["product_id"], product_id);
    assert_eq!(wo_json["bom_id"], bom_id);
    assert_eq!(wo_json["status"], "Draft");

    // POST /api/v1/manufacturing/work-orders/materials — add a material.
    let mat_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/manufacturing/work-orders/materials",
        &token,
    )
    .set_json(json!({
        "tenant_id": 1,
        "work_order_id": wo_id,
        "product_id": material_product_id,
        "quantity_required": "20.00"
    }))
    .to_request();
    let mat_resp = test::call_service(&app, mat_req).await;
    assert_eq!(
        mat_resp.status(),
        StatusCode::CREATED,
        "add work order material"
    );
    let body = to_bytes(mat_resp.into_body()).await.unwrap();
    let mat_json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(mat_json["work_order_id"], wo_id);
    assert_eq!(mat_json["product_id"], material_product_id);

    // POST /api/v1/manufacturing/work-orders/operations — add an operation.
    let op_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/manufacturing/work-orders/operations",
        &token,
    )
    .set_json(json!({
        "tenant_id": 1,
        "work_order_id": wo_id,
        "operation_sequence": 10,
        "operation_name": "Assemble",
        "work_center_id": null,
        "planned_hours": "2.00"
    }))
    .to_request();
    let op_resp = test::call_service(&app, op_req).await;
    assert_eq!(
        op_resp.status(),
        StatusCode::CREATED,
        "add work order operation"
    );
    let body = to_bytes(op_resp.into_body()).await.unwrap();
    let op_json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(op_json["work_order_id"], wo_id);
    assert_eq!(op_json["operation_name"], "Assemble");

    // GET /api/v1/manufacturing/work-orders/{id} — fetch the work order.
    let get_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/manufacturing/work-orders/{}", wo_id),
        &token,
    )
    .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), StatusCode::OK, "get work order");
    let body = to_bytes(get_resp.into_body()).await.unwrap();
    let get_json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(get_json["id"], wo_id);

    // GET /api/v1/manufacturing/work-orders — list (paginated).
    let list_req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/manufacturing/work-orders?page=1&per_page=10",
        &token,
    )
    .to_request();
    let list_resp = test::call_service(&app, list_req).await;
    assert_eq!(list_resp.status(), StatusCode::OK, "list work orders");
    let body = to_bytes(list_resp.into_body()).await.unwrap();
    let list_json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(list_json["total"], 1);
    assert!(list_json["items"]
        .as_array()
        .unwrap()
        .iter()
        .any(|w| w["id"] == wo_id));

    // GET /api/v1/manufacturing/work-orders/{work_order_id}/materials — list materials.
    let mats_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/manufacturing/work-orders/{}/materials", wo_id),
        &token,
    )
    .to_request();
    let mats_resp = test::call_service(&app, mats_req).await;
    assert_eq!(
        mats_resp.status(),
        StatusCode::OK,
        "get work order materials"
    );
    let body = to_bytes(mats_resp.into_body()).await.unwrap();
    let mats_json: Value = serde_json::from_slice(&body).unwrap();
    let mats = mats_json.as_array().expect("materials is array");
    assert_eq!(mats.len(), 1);
    assert_eq!(mats[0]["product_id"], material_product_id);

    // GET /api/v1/manufacturing/work-orders/{work_order_id}/operations — list operations.
    let ops_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/manufacturing/work-orders/{}/operations", wo_id),
        &token,
    )
    .to_request();
    let ops_resp = test::call_service(&app, ops_req).await;
    assert_eq!(
        ops_resp.status(),
        StatusCode::OK,
        "get work order operations"
    );
    let body = to_bytes(ops_resp.into_body()).await.unwrap();
    let ops_json: Value = serde_json::from_slice(&body).unwrap();
    let ops = ops_json.as_array().expect("operations is array");
    assert_eq!(ops.len(), 1);
    assert_eq!(ops[0]["operation_name"], "Assemble");

    // PUT /api/v1/manufacturing/work-orders/{id}/status — update status.
    let status_req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/manufacturing/work-orders/{}/status", wo_id),
        &token,
    )
    .set_json(json!({ "status": "InProgress" }))
    .to_request();
    let status_resp = test::call_service(&app, status_req).await;
    assert_eq!(
        status_resp.status(),
        StatusCode::OK,
        "update work order status"
    );
    let body = to_bytes(status_resp.into_body()).await.unwrap();
    let status_json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(status_json["status"], "InProgress");

    // DELETE /api/v1/manufacturing/work-orders/{id} — soft delete.
    let del_req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/manufacturing/work-orders/{}", wo_id),
        &token,
    )
    .to_request();
    let del_resp = test::call_service(&app, del_req).await;
    assert_eq!(del_resp.status(), StatusCode::OK, "soft delete work order");

    // Verify hidden.
    let get_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/manufacturing/work-orders/{}", wo_id),
        &token,
    )
    .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(
        get_resp.status(),
        StatusCode::NOT_FOUND,
        "deleted work order is 404"
    );

    // GET /api/v1/manufacturing/work-orders/deleted — list soft-deleted.
    let deleted_req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/manufacturing/work-orders/deleted",
        &token,
    )
    .to_request();
    let deleted_resp = test::call_service(&app, deleted_req).await;
    assert_eq!(
        deleted_resp.status(),
        StatusCode::OK,
        "list deleted work orders"
    );
    let body = to_bytes(deleted_resp.into_body()).await.unwrap();
    let deleted_json: Value = serde_json::from_slice(&body).unwrap();
    let deleted = deleted_json
        .as_array()
        .expect("deleted work orders is array");
    assert!(
        deleted.iter().any(|w| w["id"] == wo_id),
        "deleted list has work order"
    );

    // PUT /api/v1/manufacturing/work-orders/{id}/restore — restore.
    let restore_req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/manufacturing/work-orders/{}/restore", wo_id),
        &token,
    )
    .to_request();
    let restore_resp = test::call_service(&app, restore_req).await;
    assert_eq!(restore_resp.status(), StatusCode::OK, "restore work order");
    let body = to_bytes(restore_resp.into_body()).await.unwrap();
    let restore_json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(restore_json["id"], wo_id);

    // Verify visible again.
    let get_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/manufacturing/work-orders/{}", wo_id),
        &token,
    )
    .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(
        get_resp.status(),
        StatusCode::OK,
        "restored work order is visible"
    );

    // Soft-delete again before destroy.
    let del_req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/manufacturing/work-orders/{}", wo_id),
        &token,
    )
    .to_request();
    test::call_service(&app, del_req).await;

    // DELETE /api/v1/manufacturing/work-orders/{id}/destroy — permanently destroy.
    let destroy_req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/manufacturing/work-orders/{}/destroy", wo_id),
        &token,
    )
    .to_request();
    let destroy_resp = test::call_service(&app, destroy_req).await;
    assert_eq!(
        destroy_resp.status(),
        StatusCode::NO_CONTENT,
        "destroy work order"
    );

    let restore_req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/manufacturing/work-orders/{}/restore", wo_id),
        &token,
    )
    .to_request();
    let restore_resp = test::call_service(&app, restore_req).await;
    assert_eq!(
        restore_resp.status(),
        StatusCode::NOT_FOUND,
        "destroyed work order not restorable"
    );
}

// ============================================================================
// Inspections Workflow
// ============================================================================

/// Full inspection lifecycle: create product → create inspection → list → get →
/// update → soft delete → restore → deleted list → destroy.
#[actix_web::test]
async fn e2e_manufacturing_inspections_workflow() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, user_id) = register_admin(&state, 1).await;
    let product_id = seed_product!(&app, &token, 1);

    // POST /api/v1/manufacturing/inspections — create an inspection.
    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/manufacturing/inspections",
        &token,
    )
    .set_json(json!({
        "tenant_id": 1,
        "work_order_id": null,
        "product_id": product_id,
        "inspection_type": "Visual",
        "quantity_inspected": "100.00",
        "quantity_passed": "95.00",
        "quantity_failed": "5.00",
        "status": "Pending",
        "inspector_id": null,
        "notes": "E2E inspection"
    }))
    .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    assert_eq!(
        create_resp.status(),
        StatusCode::CREATED,
        "create inspection"
    );
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let insp_json: Value = serde_json::from_slice(&body).unwrap();
    let insp_id = extract_id(&insp_json);
    assert_eq!(insp_json["product_id"], product_id);
    assert_eq!(insp_json["inspection_type"], "Visual");
    assert_eq!(insp_json["status"], "Pending");

    // GET /api/v1/manufacturing/inspections — list inspections.
    let list_req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/manufacturing/inspections",
        &token,
    )
    .to_request();
    let list_resp = test::call_service(&app, list_req).await;
    assert_eq!(list_resp.status(), StatusCode::OK, "list inspections");
    let body = to_bytes(list_resp.into_body()).await.unwrap();
    let list_json: Value = serde_json::from_slice(&body).unwrap();
    let items = list_json.as_array().expect("inspections is array");
    assert!(
        items.iter().any(|i| i["id"] == insp_id),
        "list contains inspection"
    );

    // GET /api/v1/manufacturing/inspections/{id} — get the inspection.
    let get_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/manufacturing/inspections/{}", insp_id),
        &token,
    )
    .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), StatusCode::OK, "get inspection");
    let body = to_bytes(get_resp.into_body()).await.unwrap();
    let get_json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(get_json["id"], insp_id);

    // PUT /api/v1/manufacturing/inspections/{id} — update the inspection.
    let update_req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/manufacturing/inspections/{}", insp_id),
        &token,
    )
    .set_json(json!({
        "status": "Passed",
        "quantity_passed": "98.00",
        "quantity_failed": "2.00",
        "inspector_id": user_id,
        "notes": "E2E updated findings"
    }))
    .to_request();
    let update_resp = test::call_service(&app, update_req).await;
    assert_eq!(update_resp.status(), StatusCode::OK, "update inspection");
    let body = to_bytes(update_resp.into_body()).await.unwrap();
    let update_json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(update_json["status"], "Passed");
    assert_eq!(update_json["quantity_passed"], "98.00");
    assert_eq!(update_json["quantity_failed"], "2.00");
    assert_eq!(update_json["inspector_id"], user_id);
    assert_eq!(update_json["notes"], "E2E updated findings");

    // DELETE /api/v1/manufacturing/inspections/{id} — soft delete.
    let del_req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/manufacturing/inspections/{}", insp_id),
        &token,
    )
    .to_request();
    let del_resp = test::call_service(&app, del_req).await;
    assert_eq!(del_resp.status(), StatusCode::OK, "soft delete inspection");

    // Verify hidden.
    let get_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/manufacturing/inspections/{}", insp_id),
        &token,
    )
    .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(
        get_resp.status(),
        StatusCode::NOT_FOUND,
        "deleted inspection is 404"
    );

    // GET /api/v1/manufacturing/inspections/deleted — list soft-deleted.
    let deleted_req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/manufacturing/inspections/deleted",
        &token,
    )
    .to_request();
    let deleted_resp = test::call_service(&app, deleted_req).await;
    assert_eq!(
        deleted_resp.status(),
        StatusCode::OK,
        "list deleted inspections"
    );
    let body = to_bytes(deleted_resp.into_body()).await.unwrap();
    let deleted_json: Value = serde_json::from_slice(&body).unwrap();
    let deleted = deleted_json
        .as_array()
        .expect("deleted inspections is array");
    assert!(
        deleted.iter().any(|i| i["id"] == insp_id),
        "deleted list has inspection"
    );

    // PUT /api/v1/manufacturing/inspections/{id}/restore — restore.
    let restore_req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/manufacturing/inspections/{}/restore", insp_id),
        &token,
    )
    .to_request();
    let restore_resp = test::call_service(&app, restore_req).await;
    assert_eq!(restore_resp.status(), StatusCode::OK, "restore inspection");
    let body = to_bytes(restore_resp.into_body()).await.unwrap();
    let restore_json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(restore_json["id"], insp_id);

    // Verify visible again.
    let get_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/manufacturing/inspections/{}", insp_id),
        &token,
    )
    .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(
        get_resp.status(),
        StatusCode::OK,
        "restored inspection is visible"
    );

    // Soft-delete again before destroy.
    let del_req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/manufacturing/inspections/{}", insp_id),
        &token,
    )
    .to_request();
    test::call_service(&app, del_req).await;

    // DELETE /api/v1/manufacturing/inspections/{id}/destroy — permanently destroy.
    let destroy_req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/manufacturing/inspections/{}/destroy", insp_id),
        &token,
    )
    .to_request();
    let destroy_resp = test::call_service(&app, destroy_req).await;
    assert_eq!(
        destroy_resp.status(),
        StatusCode::NO_CONTENT,
        "destroy inspection"
    );

    let restore_req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/manufacturing/inspections/{}/restore", insp_id),
        &token,
    )
    .to_request();
    let restore_resp = test::call_service(&app, restore_req).await;
    assert_eq!(
        restore_resp.status(),
        StatusCode::NOT_FOUND,
        "destroyed inspection not restorable"
    );
}

// ============================================================================
// NCRs Workflow
// ============================================================================

/// Full NCR lifecycle: create product → create NCR → list → get → update → soft
/// delete → restore → deleted list → destroy.
#[actix_web::test]
async fn e2e_manufacturing_ncrs_workflow() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;
    let product_id = seed_product!(&app, &token, 1);

    // POST /api/v1/manufacturing/ncrs — create an NCR.
    let create_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/manufacturing/ncrs",
        &token,
    )
    .set_json(json!({
        "tenant_id": 1,
        "inspection_id": null,
        "product_id": product_id,
        "ncr_type": "Minor",
        "description": "E2E surface scratch",
        "root_cause": "Handling damage",
        "corrective_action": "Improve packaging",
        "raised_by": 1
    }))
    .to_request();
    let create_resp = test::call_service(&app, create_req).await;
    assert_eq!(create_resp.status(), StatusCode::CREATED, "create ncr");
    let body = to_bytes(create_resp.into_body()).await.unwrap();
    let ncr_json: Value = serde_json::from_slice(&body).unwrap();
    let ncr_id = extract_id(&ncr_json);
    assert_eq!(ncr_json["product_id"], product_id);
    assert_eq!(ncr_json["description"], "E2E surface scratch");
    assert_eq!(ncr_json["ncr_type"], "Minor");
    assert_eq!(ncr_json["status"], "Open");

    // GET /api/v1/manufacturing/ncrs — list NCRs.
    let list_req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/manufacturing/ncrs",
        &token,
    )
    .to_request();
    let list_resp = test::call_service(&app, list_req).await;
    assert_eq!(list_resp.status(), StatusCode::OK, "list ncrs");
    let body = to_bytes(list_resp.into_body()).await.unwrap();
    let list_json: Value = serde_json::from_slice(&body).unwrap();
    let items = list_json.as_array().expect("ncrs is array");
    assert!(items.iter().any(|n| n["id"] == ncr_id), "list contains ncr");

    // GET /api/v1/manufacturing/ncrs/{id} — get the NCR.
    let get_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/manufacturing/ncrs/{}", ncr_id),
        &token,
    )
    .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), StatusCode::OK, "get ncr");
    let body = to_bytes(get_resp.into_body()).await.unwrap();
    let get_json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(get_json["id"], ncr_id);

    // PUT /api/v1/manufacturing/ncrs/{id} — update the NCR.
    let update_req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/manufacturing/ncrs/{}", ncr_id),
        &token,
    )
    .set_json(json!({
        "ncr_type": "Major",
        "description": "E2E updated description",
        "root_cause": "Machine misalignment",
        "corrective_action": "Recalibrate machine",
        "status": "CorrectiveAction"
    }))
    .to_request();
    let update_resp = test::call_service(&app, update_req).await;
    assert_eq!(update_resp.status(), StatusCode::OK, "update ncr");
    let body = to_bytes(update_resp.into_body()).await.unwrap();
    let update_json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(update_json["ncr_type"], "Major");
    assert_eq!(update_json["description"], "E2E updated description");
    assert_eq!(update_json["root_cause"], "Machine misalignment");
    assert_eq!(update_json["corrective_action"], "Recalibrate machine");
    assert_eq!(update_json["status"], "CorrectiveAction");

    // DELETE /api/v1/manufacturing/ncrs/{id} — soft delete.
    let del_req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/manufacturing/ncrs/{}", ncr_id),
        &token,
    )
    .to_request();
    let del_resp = test::call_service(&app, del_req).await;
    assert_eq!(del_resp.status(), StatusCode::OK, "soft delete ncr");

    // Verify hidden.
    let get_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/manufacturing/ncrs/{}", ncr_id),
        &token,
    )
    .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(
        get_resp.status(),
        StatusCode::NOT_FOUND,
        "deleted ncr is 404"
    );

    // GET /api/v1/manufacturing/ncrs/deleted — list soft-deleted.
    let deleted_req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/manufacturing/ncrs/deleted",
        &token,
    )
    .to_request();
    let deleted_resp = test::call_service(&app, deleted_req).await;
    assert_eq!(deleted_resp.status(), StatusCode::OK, "list deleted ncrs");
    let body = to_bytes(deleted_resp.into_body()).await.unwrap();
    let deleted_json: Value = serde_json::from_slice(&body).unwrap();
    let deleted = deleted_json.as_array().expect("deleted ncrs is array");
    assert!(
        deleted.iter().any(|n| n["id"] == ncr_id),
        "deleted list has ncr"
    );

    // PUT /api/v1/manufacturing/ncrs/{id}/restore — restore.
    let restore_req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/manufacturing/ncrs/{}/restore", ncr_id),
        &token,
    )
    .to_request();
    let restore_resp = test::call_service(&app, restore_req).await;
    assert_eq!(restore_resp.status(), StatusCode::OK, "restore ncr");
    let body = to_bytes(restore_resp.into_body()).await.unwrap();
    let restore_json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(restore_json["id"], ncr_id);

    // Verify visible again.
    let get_req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/manufacturing/ncrs/{}", ncr_id),
        &token,
    )
    .to_request();
    let get_resp = test::call_service(&app, get_req).await;
    assert_eq!(get_resp.status(), StatusCode::OK, "restored ncr is visible");

    // Soft-delete again before destroy.
    let del_req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/manufacturing/ncrs/{}", ncr_id),
        &token,
    )
    .to_request();
    test::call_service(&app, del_req).await;

    // DELETE /api/v1/manufacturing/ncrs/{id}/destroy — permanently destroy.
    let destroy_req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/manufacturing/ncrs/{}/destroy", ncr_id),
        &token,
    )
    .to_request();
    let destroy_resp = test::call_service(&app, destroy_req).await;
    assert_eq!(destroy_resp.status(), StatusCode::NO_CONTENT, "destroy ncr");

    let restore_req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/manufacturing/ncrs/{}/restore", ncr_id),
        &token,
    )
    .to_request();
    let restore_resp = test::call_service(&app, restore_req).await;
    assert_eq!(
        restore_resp.status(),
        StatusCode::NOT_FOUND,
        "destroyed ncr not restorable"
    );
}

// ============================================================================
// Material Requirements
// ============================================================================

/// Material-requirements calculation: create product → create primary BOM → add
/// BOM line → GET /api/v1/manufacturing/material-requirements/{product_id}.
/// Verifies the endpoint returns the computed per-component requirements
/// (quantity × line quantity × (1 + scrap%)).
#[actix_web::test]
async fn e2e_manufacturing_material_requirements() {
    let state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&state)).await;
    let (token, _user_id) = register_admin(&state, 1).await;

    let parent_product_id = seed_product!(&app, &token, 1);
    let component_product_id = seed_product!(&app, &token, 1);

    // Create a primary BOM for the parent product.
    let bom_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/manufacturing/boms",
        &token,
    )
    .set_json(json!({
        "tenant_id": 1,
        "product_id": parent_product_id,
        "version": "1.0",
        "is_active": true,
        "is_primary": true,
        "valid_from": null,
        "valid_to": null
    }))
    .to_request();
    let bom_resp = test::call_service(&app, bom_req).await;
    assert_eq!(
        bom_resp.status(),
        StatusCode::CREATED,
        "create bom for material req"
    );
    let body = to_bytes(bom_resp.into_body()).await.unwrap();
    let bom_json: Value = serde_json::from_slice(&body).unwrap();
    let bom_id = extract_id(&bom_json);

    // Add a BOM line: 5 units of the component, 2% scrap.
    let line_req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/manufacturing/boms/lines",
        &token,
    )
    .set_json(json!({
        "tenant_id": 1,
        "bom_id": bom_id,
        "component_product_id": component_product_id,
        "quantity": "5.00",
        "unit_id": null,
        "scrap_percentage": "2.00",
        "is_optional": false,
        "notes": null
    }))
    .to_request();
    let line_resp = test::call_service(&app, line_req).await;
    assert_eq!(
        line_resp.status(),
        StatusCode::CREATED,
        "add bom line for material req"
    );

    // GET /api/v1/manufacturing/material-requirements/{product_id}?quantity=10
    // Expected: 10 * 5 * (1 + 0.02) = 51.00
    let req = auth_request(
        actix_web::http::Method::GET,
        &format!(
            "/api/v1/manufacturing/material-requirements/{}?quantity=10",
            parent_product_id
        ),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "calculate material requirements"
    );
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let requirements = json.as_array().expect("material requirements is array");
    assert_eq!(requirements.len(), 1, "one component requirement");
    assert_eq!(
        requirements[0][0], component_product_id,
        "component product id matches"
    );
    let qty_str = requirements[0][1].as_str().unwrap_or("");
    assert!(
        qty_str.starts_with("51.0"),
        "required quantity ≈ 51 (10 * 5 * 1.02), got {}",
        qty_str
    );

    // Verify a product with no primary BOM returns an empty list.
    let orphan_product_id = seed_product!(&app, &token, 1);
    let req = auth_request(
        actix_web::http::Method::GET,
        &format!(
            "/api/v1/manufacturing/material-requirements/{}?quantity=10",
            orphan_product_id
        ),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "material requirements for product without bom"
    );
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let requirements = json.as_array().expect("material requirements is array");
    assert_eq!(
        requirements.len(),
        0,
        "no requirements without a primary bom"
    );
}
