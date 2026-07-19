//! End-to-end CRM integration tests covering all 40 CRM endpoints.
//!
//! Each workflow test exercises the full lifecycle for one CRM sub-resource
//! (leads, opportunities, campaigns, tickets): create -> list -> get ->
//! status update -> (convert | resolve) -> soft delete -> restore ->
//! deleted list -> permanent destroy. Two additional tests cover the
//! cross-resource read-only endpoints (`pipeline-value`, `open-count`).

use actix_web::{body::to_bytes, body::MessageBody, dev::ServiceResponse, http::StatusCode, test};
use serde_json::{json, Value};

use crate::common::*;

/// Helper: extract the JSON body from a service response.
async fn body_json<B: MessageBody>(resp: ServiceResponse<B>) -> Value
where
    <B as MessageBody>::Error: std::fmt::Debug,
{
    let body = to_bytes(resp.into_body()).await.unwrap();
    serde_json::from_slice(&body).unwrap_or(Value::Null)
}

/// Short unique suffix to avoid any cross-test name/code collisions.
fn uid() -> String {
    uuid::Uuid::new_v4()
        .to_string()
        .split('-')
        .next()
        .unwrap()
        .to_string()
}

// ============================================================================
// Leads — 10 endpoints
// ============================================================================

#[actix_web::test]
async fn e2e_crm_leads_full_workflow() {
    let app_state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&app_state)).await;
    let (token, _user_id) = register_admin(&app_state, 1).await;
    let suffix = uid();

    // 1. POST /api/v1/crm/leads — create
    let req = auth_request(actix_web::http::Method::POST, "/api/v1/crm/leads", &token)
        .set_json(json!({
            "tenant_id": 1,
            "name": format!("E2E Lead {}", suffix),
            "company": "E2E Corp",
            "email": format!("lead{}@e2e.test", suffix),
            "phone": "+1234567890",
            "source": "Website",
            "assigned_to": null,
            "notes": "E2E lead note"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let json = body_json(resp).await;
    let id = json["id"].as_i64().unwrap();
    assert_eq!(json["name"], format!("E2E Lead {}", suffix));
    assert_eq!(json["status"], "New");

    // 2. GET /api/v1/crm/leads — list
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/crm/leads?page=1&per_page=50",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert!(json["items"]
        .as_array()
        .unwrap()
        .iter()
        .any(|x| x["id"] == id));

    // 3. GET /api/v1/crm/leads/{id} — get by id
    let req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/crm/leads/{}", id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["id"], id);

    // 4. GET /api/v1/crm/leads/status/{status} — by status (New)
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/crm/leads/status/New?page=1&per_page=50",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert!(json["items"]
        .as_array()
        .unwrap()
        .iter()
        .any(|x| x["id"] == id));

    // 5. PUT /api/v1/crm/leads/{id}/status — update status to Contacted
    let req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/crm/leads/{}/status", id),
        &token,
    )
    .set_json(json!({ "status": "Contacted" }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["status"], "Contacted");

    // 6. POST /api/v1/crm/leads/{id}/convert — convert to customer
    let req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/crm/leads/{}/convert", id),
        &token,
    )
    .set_json(json!({ "customer_id": 1 }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["status"], "Converted");
    assert_eq!(json["converted_to_customer_id"], 1);

    // 7. DELETE /api/v1/crm/leads/{id} — soft delete
    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/crm/leads/{}", id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Verify it's gone from the active list/get
    let req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/crm/leads/{}", id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    // 8. GET /api/v1/crm/leads/deleted — deleted list
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/crm/leads/deleted",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert!(json.as_array().unwrap().iter().any(|x| x["id"] == id));

    // 9. PUT /api/v1/crm/leads/{id}/restore — restore
    let req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/crm/leads/{}/restore", id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["id"], id);

    // Soft-delete again before permanent destroy
    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/crm/leads/{}", id),
        &token,
    )
    .to_request();
    test::call_service(&app, req).await;

    // 10. DELETE /api/v1/crm/leads/{id}/destroy — permanent destroy
    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/crm/leads/{}/destroy", id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // Restore should now 404
    let req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/crm/leads/{}/restore", id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

// ============================================================================
// Opportunities — 10 endpoints
// ============================================================================

#[actix_web::test]
async fn e2e_crm_opportunities_full_workflow() {
    let app_state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&app_state)).await;
    let (token, _user_id) = register_admin(&app_state, 1).await;
    let suffix = uid();

    // 1. POST /api/v1/crm/opportunities — create
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/crm/opportunities",
        &token,
    )
    .set_json(json!({
        "tenant_id": 1,
        "lead_id": null,
        "name": format!("E2E Opp {}", suffix),
        "customer_id": null,
        "value": "50000.00",
        "probability": "50",
        "expected_close_date": null,
        "assigned_to": null,
        "notes": "E2E opportunity"
    }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let json = body_json(resp).await;
    let id = json["id"].as_i64().unwrap();
    assert_eq!(json["name"], format!("E2E Opp {}", suffix));
    assert_eq!(json["status"], "Open");

    // 2. GET /api/v1/crm/opportunities — list
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/crm/opportunities?page=1&per_page=50",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert!(json["items"]
        .as_array()
        .unwrap()
        .iter()
        .any(|x| x["id"] == id));

    // 3. GET /api/v1/crm/opportunities/{id} — get by id
    let req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/crm/opportunities/{}", id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["id"], id);

    // 4. GET /api/v1/crm/opportunities/status/{status} — by status (Open)
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/crm/opportunities/status/Open?page=1&per_page=50",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert!(json["items"]
        .as_array()
        .unwrap()
        .iter()
        .any(|x| x["id"] == id));

    // 5. PUT /api/v1/crm/opportunities/{id}/status — update status to Won
    let req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/crm/opportunities/{}/status", id),
        &token,
    )
    .set_json(json!({ "status": "Won" }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["status"], "Won");

    // 6. DELETE /api/v1/crm/opportunities/{id} — soft delete
    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/crm/opportunities/{}", id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // 7. GET /api/v1/crm/opportunities/deleted — deleted list
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/crm/opportunities/deleted",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert!(json.as_array().unwrap().iter().any(|x| x["id"] == id));

    // 8. PUT /api/v1/crm/opportunities/{id}/restore — restore
    let req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/crm/opportunities/{}/restore", id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["id"], id);

    // Soft-delete again before permanent destroy
    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/crm/opportunities/{}", id),
        &token,
    )
    .to_request();
    test::call_service(&app, req).await;

    // 9. DELETE /api/v1/crm/opportunities/{id}/destroy — permanent destroy
    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/crm/opportunities/{}/destroy", id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // Restore should now 404
    let req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/crm/opportunities/{}/restore", id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

// ============================================================================
// Campaigns — 9 endpoints
// ============================================================================

#[actix_web::test]
async fn e2e_crm_campaigns_full_workflow() {
    let app_state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&app_state)).await;
    let (token, _user_id) = register_admin(&app_state, 1).await;
    let suffix = uid();

    // 1. POST /api/v1/crm/campaigns — create
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/crm/campaigns",
        &token,
    )
    .set_json(json!({
        "tenant_id": 1,
        "name": format!("E2E Campaign {}", suffix),
        "description": "E2E campaign description",
        "campaign_type": "Email",
        "budget": "10000.00",
        "start_date": null,
        "end_date": null
    }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let json = body_json(resp).await;
    let id = json["id"].as_i64().unwrap();
    assert_eq!(json["name"], format!("E2E Campaign {}", suffix));
    assert_eq!(json["status"], "Draft");

    // 2. GET /api/v1/crm/campaigns — list
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/crm/campaigns?page=1&per_page=50",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert!(json["items"]
        .as_array()
        .unwrap()
        .iter()
        .any(|x| x["id"] == id));

    // 3. GET /api/v1/crm/campaigns/{id} — get by id
    let req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/crm/campaigns/{}", id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["id"], id);

    // 4. GET /api/v1/crm/campaigns/status/{status} — by status (Draft)
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/crm/campaigns/status/Draft?page=1&per_page=50",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert!(json["items"]
        .as_array()
        .unwrap()
        .iter()
        .any(|x| x["id"] == id));

    // 5. PUT /api/v1/crm/campaigns/{id}/status — update status to Active
    let req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/crm/campaigns/{}/status", id),
        &token,
    )
    .set_json(json!({ "status": "Active" }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["status"], "Active");

    // 6. DELETE /api/v1/crm/campaigns/{id} — soft delete
    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/crm/campaigns/{}", id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // 7. GET /api/v1/crm/campaigns/deleted — deleted list
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/crm/campaigns/deleted",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert!(json.as_array().unwrap().iter().any(|x| x["id"] == id));

    // 8. PUT /api/v1/crm/campaigns/{id}/restore — restore
    let req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/crm/campaigns/{}/restore", id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["id"], id);

    // Soft-delete again before permanent destroy
    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/crm/campaigns/{}", id),
        &token,
    )
    .to_request();
    test::call_service(&app, req).await;

    // 9. DELETE /api/v1/crm/campaigns/{id}/destroy — permanent destroy
    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/crm/campaigns/{}/destroy", id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // Restore should now 404
    let req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/crm/campaigns/{}/restore", id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

// ============================================================================
// Tickets — 11 endpoints
// ============================================================================

#[actix_web::test]
async fn e2e_crm_tickets_full_workflow() {
    let app_state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&app_state)).await;
    let (token, _user_id) = register_admin(&app_state, 1).await;
    let suffix = uid();

    // 1. POST /api/v1/crm/tickets — create
    let req = auth_request(actix_web::http::Method::POST, "/api/v1/crm/tickets", &token)
        .set_json(json!({
            "tenant_id": 1,
            "subject": format!("E2E Ticket {}", suffix),
            "description": "E2E ticket description",
            "customer_id": null,
            "assigned_to": null,
            "priority": "High",
            "category": "Technical"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let json = body_json(resp).await;
    let id = json["id"].as_i64().unwrap();
    assert_eq!(json["subject"], format!("E2E Ticket {}", suffix));
    assert_eq!(json["status"], "Open");
    assert_eq!(json["priority"], "High");

    // 2. GET /api/v1/crm/tickets — list
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/crm/tickets?page=1&per_page=50",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert!(json["items"]
        .as_array()
        .unwrap()
        .iter()
        .any(|x| x["id"] == id));

    // 3. GET /api/v1/crm/tickets/{id} — get by id
    let req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/crm/tickets/{}", id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["id"], id);

    // 4. GET /api/v1/crm/tickets/status/{status} — by status (Open)
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/crm/tickets/status/Open?page=1&per_page=50",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert!(json["items"]
        .as_array()
        .unwrap()
        .iter()
        .any(|x| x["id"] == id));

    // 5. POST /api/v1/crm/tickets/{id}/resolve — resolve ticket
    let req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/crm/tickets/{}/resolve", id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["status"], "Resolved");
    assert!(json["resolved_at"].is_string());

    // 6. PUT /api/v1/crm/tickets/{id}/status — update status to Closed
    let req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/crm/tickets/{}/status", id),
        &token,
    )
    .set_json(json!({ "status": "Closed" }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["status"], "Closed");

    // 7. DELETE /api/v1/crm/tickets/{id} — soft delete
    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/crm/tickets/{}", id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    // 8. GET /api/v1/crm/tickets/deleted — deleted list
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/crm/tickets/deleted",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert!(json.as_array().unwrap().iter().any(|x| x["id"] == id));

    // 9. PUT /api/v1/crm/tickets/{id}/restore — restore
    let req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/crm/tickets/{}/restore", id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert_eq!(json["id"], id);

    // Soft-delete again before permanent destroy
    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/crm/tickets/{}", id),
        &token,
    )
    .to_request();
    test::call_service(&app, req).await;

    // 10. DELETE /api/v1/crm/tickets/{id}/destroy — permanent destroy
    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/crm/tickets/{}/destroy", id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT);

    // Restore should now 404
    let req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/crm/tickets/{}/restore", id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

// ============================================================================
// Cross-resource read-only endpoints
// ============================================================================

#[actix_web::test]
async fn e2e_crm_pipeline_value() {
    let app_state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&app_state)).await;
    let (token, _user_id) = register_admin(&app_state, 1).await;
    let suffix = uid();

    // Seed an Open opportunity so the pipeline value is non-zero.
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/crm/opportunities",
        &token,
    )
    .set_json(json!({
        "tenant_id": 1,
        "lead_id": null,
        "name": format!("Pipeline Opp {}", suffix),
        "customer_id": null,
        "value": "50000.00",
        "probability": "50",
        "expected_close_date": null,
        "assigned_to": null,
        "notes": null
    }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    // GET /api/v1/crm/pipeline-value
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/crm/pipeline-value",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    assert!(json.get("pipeline_value").is_some());
}

#[actix_web::test]
async fn e2e_crm_tickets_open_count() {
    let app_state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&app_state)).await;
    let (token, _user_id) = register_admin(&app_state, 1).await;
    let suffix = uid();

    // Seed an Open ticket.
    let req = auth_request(actix_web::http::Method::POST, "/api/v1/crm/tickets", &token)
        .set_json(json!({
            "tenant_id": 1,
            "subject": format!("Open Count Ticket {}", suffix),
            "description": "Should be counted as open",
            "customer_id": null,
            "assigned_to": null,
            "priority": "Medium",
            "category": "Support"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    // GET /api/v1/crm/tickets/open-count
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/crm/tickets/open-count",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let json = body_json(resp).await;
    let count = json["open_tickets_count"].as_i64().unwrap();
    assert!(count >= 1);
}
