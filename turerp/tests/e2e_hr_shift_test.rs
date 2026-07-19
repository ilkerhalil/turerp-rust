//! End-to-End HR & Shift Planning Integration Tests
//!
//! Exercises all 35 HR endpoints and all 18 Shift Planning endpoints through
//! the full HTTP stack (actix test::call_service) against the in-memory
//! backend. Each test function builds its own fresh AppState so tests are
//! isolated from one another.
//!
//! Run with: `cargo test --test integration e2e_hr_shift`

use actix_web::{body::to_bytes, http::StatusCode, test, web, App};
use serde_json::{json, Value};

use crate::common::*;

use turerp::api::v1_shifts_configure;
use turerp::middleware::JwtAuthMiddleware;

// ---------------------------------------------------------------------------
// Shift-enabled test app
// ---------------------------------------------------------------------------
// The shared `build_test_app` helper in `tests/common/mod.rs` does not register
// the Shift Planning routes (they were added after the common helper was
// frozen). We therefore build a local variant that layers `v1_shifts_configure`
// and the `shift_service` app_data on top of the same configuration used by the
// rest of the suite. This mirrors the approach in `tests/shift_crud_test.rs`.

fn build_test_app_with_shift(
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
        .wrap(JwtAuthMiddleware::new(jwt))
        .app_data(web::Data::new(state.clone()))
        .app_data(state.auth.auth_service.clone())
        .app_data(state.auth.user_service.clone())
        .app_data(state.auth.jwt_service.clone())
        .app_data(state.commerce.cari_service.clone())
        .app_data(state.commerce.stock_service.clone())
        .app_data(state.commerce.invoice_service.clone())
        .app_data(state.commerce.sales_service.clone())
        .app_data(state.hr.hr_service.clone())
        .app_data(state.finance.accounting_service.clone())
        .app_data(state.project.project_service.clone())
        .app_data(state.project.manufacturing_service.clone())
        .app_data(state.project.crm_service.clone())
        .app_data(state.admin.tenant_service.clone())
        .app_data(state.admin.tenant_config_service.clone())
        .app_data(state.i18n.clone())
        .app_data(state.assets_service.clone())
        .app_data(state.feature_service.clone())
        .app_data(state.commerce.product_service.clone())
        .app_data(state.commerce.purchase_service.clone())
        .app_data(state.chart_of_accounts_service.clone())
        .app_data(state.finance.tax_service.clone())
        .app_data(state.integration.customer_portal_service.clone())
        .app_data(state.integration.webhook_service.clone())
        .app_data(state.infra.search_service.clone())
        .app_data(state.infra.report_engine.clone())
        .app_data(state.infra.job_scheduler.clone())
        .app_data(state.infra.notification_service.clone())
        .app_data(state.analytics.audit_service.clone())
        .app_data(state.finance.bank_service.clone())
        .app_data(state.finance.cost_center_service.clone())
        .app_data(state.document.dashboard_service.clone())
        .app_data(state.document.file_storage.clone())
        .app_data(state.project.qc_service.clone())
        .app_data(state.admin.settings_service.clone())
        .app_data(state.admin.api_key_service.clone())
        .app_data(state.analytics.subscription_service.clone())
        .app_data(state.integration.workflow_service.clone())
        .app_data(state.finance.currency_service.clone())
        .app_data(state.infra.import_service.clone())
        .app_data(state.commerce.inter_company_service.clone())
        .app_data(state.integration.efatura_service.clone())
        .app_data(state.integration.edefter_service.clone())
        .app_data(state.commerce.company_service.clone())
        .app_data(state.hr.shift_service.clone())
        .service(
            web::scope("/api")
                .configure(configure_all_routes)
                .configure(configure_v1_routes)
                .configure(v1_shifts_configure),
        )
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Create an employee and return its id. The employee is created in tenant 1
/// (the tenant the admin token is scoped to). Implemented as a macro because
/// `test::init_service` returns an opaque `impl Service` type whose generics
/// cannot easily be expressed as a function parameter.
macro_rules! create_employee {
    ($app:expr, $token:expr, $suffix:expr) => {{
        let hire_date = chrono::Utc::now();
        let req = auth_request(
            actix_web::http::Method::POST,
            "/api/v1/hr/employees",
            $token,
        )
        .set_json(json!({
            "employee_number": format!("E2E-EMP-{}", $suffix),
            "first_name": "E2E",
            "last_name": format!("Test-{}", $suffix),
            "email": format!("e2e.{}@test.com", $suffix),
            "phone": "+905550000000",
            "department": "Engineering",
            "position": "Engineer",
            "hire_date": hire_date.to_rfc3339(),
            "salary": "7500.00",
            "tc_kimlik_no": format!("1{}9", $suffix.chars().take(10).collect::<String>()),
            "children_count": 0,
            "tenant_id": 1
        }))
        .to_request();
        let resp = test::call_service($app, req).await;
        assert_eq!(resp.status(), StatusCode::CREATED, "create employee");
        let body = to_bytes(resp.into_body()).await.unwrap();
        let json: Value = serde_json::from_slice(&body).unwrap();
        json["id"].as_i64().unwrap()
    }};
}

/// Parse the JSON body of a response into a `serde_json::Value`. Generic over
/// the body type so it works with both `BoxBody` (HR app) and
/// `EitherBody<BoxBody>` (shift app, which adds the JWT middleware wrapper).
async fn body_json<B>(resp: actix_web::dev::ServiceResponse<B>) -> Value
where
    B: actix_web::body::MessageBody,
    <B as actix_web::body::MessageBody>::Error: std::fmt::Debug,
{
    let body = to_bytes(resp.into_body()).await.unwrap();
    serde_json::from_slice(&body).unwrap_or(Value::Null)
}

// ---------------------------------------------------------------------------
// HR: Employees full workflow
// ---------------------------------------------------------------------------

#[actix_web::test]
async fn e2e_hr_employees_full_workflow() {
    let app_state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&app_state)).await;
    let (token, _user_id) = register_admin(&app_state, 1).await;

    // POST /api/v1/hr/employees — create
    let id = create_employee!(&app, &token, "emp001");

    // GET /api/v1/hr/employees — list
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/hr/employees?page=1&per_page=10",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "list employees");
    let json = body_json(resp).await;
    let items = json["items"].as_array().unwrap();
    assert!(items.iter().any(|e| e["id"].as_i64() == Some(id)));

    // GET /api/v1/hr/employees/{id} — get
    let req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/hr/employees/{}", id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "get employee");
    let json = body_json(resp).await;
    assert_eq!(json["id"], id);
    assert_eq!(json["status"], "Active");

    // PUT /api/v1/hr/employees/{id}/status — update status
    let req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/hr/employees/{}/status", id),
        &token,
    )
    .set_json(json!({ "status": "Suspended" }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "update status");
    let json = body_json(resp).await;
    assert_eq!(json["status"], "Suspended");

    // POST /api/v1/hr/employees/{id}/terminate — terminate
    let req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/hr/employees/{}/terminate", id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "terminate employee");
    let json = body_json(resp).await;
    assert_eq!(json["status"], "Terminated");

    // DELETE /api/v1/hr/employees/{id} — soft delete (note: no /soft-delete
    // suffix for employees; the DELETE on the resource maps to soft_delete).
    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/hr/employees/{}", id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::NO_CONTENT,
        "soft delete employee"
    );

    // GET /api/v1/hr/employees/{id} — should be 404 after soft delete
    let req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/hr/employees/{}", id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::NOT_FOUND,
        "get after soft delete"
    );

    // GET /api/v1/hr/employees/deleted — deleted list
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/hr/employees/deleted",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "list deleted employees");
    let json = body_json(resp).await;
    let items = json.as_array().unwrap();
    assert!(items.iter().any(|e| e["id"].as_i64() == Some(id)));

    // PUT /api/v1/hr/employees/{id}/restore — restore
    let req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/hr/employees/{}/restore", id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "restore employee");
    let json = body_json(resp).await;
    assert_eq!(json["id"], id);

    // Soft delete again then permanently destroy
    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/hr/employees/{}", id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::NO_CONTENT,
        "soft delete before destroy"
    );

    // DELETE /api/v1/hr/employees/{id}/destroy — permanent destroy
    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/hr/employees/{}/destroy", id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT, "destroy employee");

    // Restore should now fail (404)
    let req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/hr/employees/{}/restore", id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::NOT_FOUND,
        "restore after destroy"
    );
}

// ---------------------------------------------------------------------------
// HR: Leave types workflow
// ---------------------------------------------------------------------------
// Leave types are seeded by the in-memory repository (ids 1-3 for tenant 1);
// there is no POST endpoint to create them. This workflow exercises list →
// soft delete → deleted list → restore → destroy.

#[actix_web::test]
async fn e2e_hr_leave_types_workflow() {
    let app_state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&app_state)).await;
    let (token, _user_id) = register_admin(&app_state, 1).await;

    // GET /api/v1/hr/leave-types — list (seeded: 3 default types)
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/hr/leave-types",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "list leave types");
    let json = body_json(resp).await;
    let items = json.as_array().unwrap();
    assert!(items.len() >= 3, "seeded leave types present");
    let id = items[0]["id"].as_i64().unwrap();

    // GET /api/v1/hr/leave-types/deleted — should be empty initially
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/hr/leave-types/deleted",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "list deleted leave types (empty)"
    );
    let json = body_json(resp).await;
    assert!(json.as_array().unwrap().is_empty());

    // DELETE /api/v1/hr/leave-types/{id}/soft-delete — soft delete
    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/hr/leave-types/{}/soft-delete", id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::NO_CONTENT,
        "soft delete leave type"
    );

    // GET /api/v1/hr/leave-types/deleted — should now contain the deleted type
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/hr/leave-types/deleted",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "list deleted leave types (populated)"
    );
    let json = body_json(resp).await;
    let items = json.as_array().unwrap();
    assert!(items.iter().any(|lt| lt["id"].as_i64() == Some(id)));

    // PUT /api/v1/hr/leave-types/{id}/restore — restore
    let req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/hr/leave-types/{}/restore", id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "restore leave type");
    let json = body_json(resp).await;
    assert_eq!(json["id"], id);

    // Soft delete again, then permanently destroy
    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/hr/leave-types/{}/soft-delete", id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::NO_CONTENT,
        "soft delete before destroy"
    );

    // DELETE /api/v1/hr/leave-types/{id}/destroy — permanent destroy
    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/hr/leave-types/{}/destroy", id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT, "destroy leave type");

    // Restore should now fail (404)
    let req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/hr/leave-types/{}/restore", id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::NOT_FOUND,
        "restore after destroy"
    );
}

// ---------------------------------------------------------------------------
// HR: Leave requests workflow
// ---------------------------------------------------------------------------

#[actix_web::test]
async fn e2e_hr_leave_requests_workflow() {
    let app_state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&app_state)).await;
    let (token, _user_id) = register_admin(&app_state, 1).await;

    // Create an employee to own the leave requests
    let employee_id = create_employee!(&app, &token, "leaveemp");

    // The in-memory repository seeds leave type id=1 ("Annual Leave") for
    // tenant 1; use it as the leave_type_id.
    let leave_type_id: i64 = 1;

    // POST /api/v1/hr/leave-requests — create (approve flow)
    let start = chrono::Utc::now();
    let end = start + chrono::Duration::days(3);
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/hr/leave-requests",
        &token,
    )
    .set_json(json!({
        "employee_id": employee_id,
        "leave_type_id": leave_type_id,
        "start_date": start.to_rfc3339(),
        "end_date": end.to_rfc3339(),
        "reason": "E2E vacation",
        "tenant_id": 1
    }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::CREATED,
        "create leave request (approve)"
    );
    let json = body_json(resp).await;
    let approve_id = json["id"].as_i64().unwrap();
    assert_eq!(json["status"], "Pending");

    // POST /api/v1/hr/leave-requests/{id}/approve — approve
    let req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/hr/leave-requests/{}/approve", approve_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "approve leave request");
    let json = body_json(resp).await;
    assert_eq!(json["status"], "Approved");

    // POST /api/v1/hr/leave-requests — create a second request (reject flow)
    let start2 = chrono::Utc::now() + chrono::Duration::days(10);
    let end2 = start2 + chrono::Duration::days(1);
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/hr/leave-requests",
        &token,
    )
    .set_json(json!({
        "employee_id": employee_id,
        "leave_type_id": leave_type_id,
        "start_date": start2.to_rfc3339(),
        "end_date": end2.to_rfc3339(),
        "reason": "E2E personal",
        "tenant_id": 1
    }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::CREATED,
        "create leave request (reject)"
    );
    let json = body_json(resp).await;
    let reject_id = json["id"].as_i64().unwrap();

    // POST /api/v1/hr/leave-requests/{id}/reject — reject
    let req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/hr/leave-requests/{}/reject", reject_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "reject leave request");
    let json = body_json(resp).await;
    assert_eq!(json["status"], "Rejected");

    // GET /api/v1/hr/leave-requests/employee/{employee_id} — list by employee
    let req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/hr/leave-requests/employee/{}", employee_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "list leave requests by employee"
    );
    let json = body_json(resp).await;
    let items = json.as_array().unwrap();
    assert_eq!(items.len(), 2, "two leave requests for employee");

    // GET /api/v1/hr/leave-requests/deleted — should be empty
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/hr/leave-requests/deleted",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "list deleted leave requests (empty)"
    );
    let json = body_json(resp).await;
    assert!(json.as_array().unwrap().is_empty());

    // DELETE /api/v1/hr/leave-requests/{id}/soft-delete — soft delete the approved one
    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/hr/leave-requests/{}/soft-delete", approve_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::NO_CONTENT,
        "soft delete leave request"
    );

    // GET /api/v1/hr/leave-requests/deleted — should now contain it
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/hr/leave-requests/deleted",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "list deleted leave requests (populated)"
    );
    let json = body_json(resp).await;
    let items = json.as_array().unwrap();
    assert!(items.iter().any(|lr| lr["id"].as_i64() == Some(approve_id)));

    // PUT /api/v1/hr/leave-requests/{id}/restore — restore
    let req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/hr/leave-requests/{}/restore", approve_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "restore leave request");
    let json = body_json(resp).await;
    assert_eq!(json["id"], approve_id);

    // Soft delete again then permanently destroy
    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/hr/leave-requests/{}/soft-delete", approve_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::NO_CONTENT,
        "soft delete before destroy"
    );

    // DELETE /api/v1/hr/leave-requests/{id}/destroy — permanent destroy
    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/hr/leave-requests/{}/destroy", approve_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::NO_CONTENT,
        "destroy leave request"
    );

    // Restore should now fail (404)
    let req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/hr/leave-requests/{}/restore", approve_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::NOT_FOUND,
        "restore after destroy"
    );
}

// ---------------------------------------------------------------------------
// HR: Attendance workflow
// ---------------------------------------------------------------------------

#[actix_web::test]
async fn e2e_hr_attendance_workflow() {
    let app_state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&app_state)).await;
    let (token, _user_id) = register_admin(&app_state, 1).await;

    // Create an employee to own the attendance records
    let employee_id = create_employee!(&app, &token, "attndemp");

    // POST /api/v1/hr/attendance — record attendance
    let check_in = chrono::Utc::now();
    let check_out = check_in + chrono::Duration::hours(8);
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/hr/attendance",
        &token,
    )
    .set_json(json!({
        "employee_id": employee_id,
        "date": check_in.to_rfc3339(),
        "check_in": check_in.to_rfc3339(),
        "check_out": check_out.to_rfc3339(),
        "notes": "E2E attendance",
        "tenant_id": 1
    }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED, "record attendance");
    let json = body_json(resp).await;
    let id = json["id"].as_i64().unwrap();
    assert_eq!(json["employee_id"], employee_id);

    // GET /api/v1/hr/attendance/employee/{employee_id} — list by employee
    let req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/hr/attendance/employee/{}", employee_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "list attendance by employee");
    let json = body_json(resp).await;
    let items = json.as_array().unwrap();
    assert!(items.iter().any(|a| a["id"].as_i64() == Some(id)));

    // GET /api/v1/hr/attendance/deleted — should be empty
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/hr/attendance/deleted",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "list deleted attendance (empty)"
    );
    let json = body_json(resp).await;
    assert!(json.as_array().unwrap().is_empty());

    // DELETE /api/v1/hr/attendance/{id}/soft-delete — soft delete
    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/hr/attendance/{}/soft-delete", id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::NO_CONTENT,
        "soft delete attendance"
    );

    // GET /api/v1/hr/attendance/deleted — should now contain it
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/hr/attendance/deleted",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "list deleted attendance (populated)"
    );
    let json = body_json(resp).await;
    let items = json.as_array().unwrap();
    assert!(items.iter().any(|a| a["id"].as_i64() == Some(id)));

    // PUT /api/v1/hr/attendance/{id}/restore — restore
    let req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/hr/attendance/{}/restore", id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "restore attendance");
    let json = body_json(resp).await;
    assert_eq!(json["id"], id);

    // Soft delete again then permanently destroy
    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/hr/attendance/{}/soft-delete", id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::NO_CONTENT,
        "soft delete before destroy"
    );

    // DELETE /api/v1/hr/attendance/{id}/destroy — permanent destroy
    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/hr/attendance/{}/destroy", id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT, "destroy attendance");

    // Restore should now fail (404)
    let req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/hr/attendance/{}/restore", id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::NOT_FOUND,
        "restore after destroy"
    );
}

// ---------------------------------------------------------------------------
// HR: Payroll workflow
// ---------------------------------------------------------------------------

#[actix_web::test]
async fn e2e_hr_payroll_workflow() {
    let app_state = create_test_app_state().await;
    let app = test::init_service(build_test_app(&app_state)).await;
    let (token, _user_id) = register_admin(&app_state, 1).await;

    // Create an employee to own the payroll records
    let employee_id = create_employee!(&app, &token, "payrollemp");

    // POST /api/v1/hr/payroll/calculate — calculate payroll
    let period_start = chrono::Utc::now() - chrono::Duration::days(30);
    let period_end = chrono::Utc::now();
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/hr/payroll/calculate",
        &token,
    )
    .set_json(json!({
        "employee_id": employee_id,
        "period_start": period_start.to_rfc3339(),
        "period_end": period_end.to_rfc3339()
    }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "calculate payroll");
    let json = body_json(resp).await;
    let payroll_id = json["id"].as_i64().unwrap();
    assert_eq!(json["employee_id"], employee_id);
    assert_eq!(json["status"], "Calculated");

    // POST /api/v1/hr/payroll/{id}/paid — mark paid
    let req = auth_request(
        actix_web::http::Method::POST,
        &format!("/api/v1/hr/payroll/{}/paid", payroll_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "mark payroll paid");
    let json = body_json(resp).await;
    assert_eq!(json["id"], payroll_id);
    assert_eq!(json["status"], "Paid");

    // GET /api/v1/hr/payroll/employee/{employee_id} — list by employee
    let req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/hr/payroll/employee/{}", employee_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "list payroll by employee");
    let json = body_json(resp).await;
    let items = json.as_array().unwrap();
    assert!(items.iter().any(|p| p["id"].as_i64() == Some(payroll_id)));

    // GET /api/v1/hr/payroll/deleted — should be empty
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/hr/payroll/deleted",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "list deleted payroll (empty)"
    );
    let json = body_json(resp).await;
    assert!(json.as_array().unwrap().is_empty());

    // DELETE /api/v1/hr/payroll/{id}/soft-delete — soft delete
    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/hr/payroll/{}/soft-delete", payroll_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT, "soft delete payroll");

    // GET /api/v1/hr/payroll/deleted — should now contain it
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/hr/payroll/deleted",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "list deleted payroll (populated)"
    );
    let json = body_json(resp).await;
    let items = json.as_array().unwrap();
    assert!(items.iter().any(|p| p["id"].as_i64() == Some(payroll_id)));

    // PUT /api/v1/hr/payroll/{id}/restore — restore
    let req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/hr/payroll/{}/restore", payroll_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "restore payroll");
    let json = body_json(resp).await;
    assert_eq!(json["id"], payroll_id);

    // Soft delete again then permanently destroy
    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/hr/payroll/{}/soft-delete", payroll_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::NO_CONTENT,
        "soft delete before destroy"
    );

    // DELETE /api/v1/hr/payroll/{id}/destroy — permanent destroy
    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/hr/payroll/{}/destroy", payroll_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT, "destroy payroll");

    // Restore should now fail (404)
    let req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/hr/payroll/{}/restore", payroll_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::NOT_FOUND,
        "restore after destroy"
    );
}

// ---------------------------------------------------------------------------
// Shift Planning: full workflow
// ---------------------------------------------------------------------------

#[actix_web::test]
async fn e2e_shifts_full_workflow() {
    let app_state = create_test_app_state().await;
    let app = test::init_service(build_test_app_with_shift(&app_state)).await;
    let (token, _user_id) = register_admin(&app_state, 1).await;

    // Create an employee to use in assignments / attendance / overtime.
    let employee_id = create_employee!(&app, &token, "shiftemp");

    // POST /api/v1/shifts — create shift
    let req = auth_request(actix_web::http::Method::POST, "/api/v1/shifts", &token)
        .set_json(json!({
            "tenant_id": 1,
            "name": "E2E Morning Shift",
            "shift_type": "Morning",
            "start_time": "08:00:00",
            "end_time": "16:00:00",
            "break_duration_minutes": 60,
            "expected_hours": "8.00"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED, "create shift");
    let json = body_json(resp).await;
    let shift_id = json["id"].as_i64().unwrap();
    assert_eq!(json["name"], "E2E Morning Shift");
    assert_eq!(json["shift_type"], "Morning");
    assert_eq!(json["is_active"], true);

    // GET /api/v1/shifts — list
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/shifts?page=1&per_page=10",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "list shifts");
    let json = body_json(resp).await;
    let items = json["items"].as_array().unwrap();
    assert!(items.iter().any(|s| s["id"].as_i64() == Some(shift_id)));

    // GET /api/v1/shifts/{id} — get
    let req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/shifts/{}", shift_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "get shift");
    let json = body_json(resp).await;
    assert_eq!(json["id"], shift_id);
    assert_eq!(json["name"], "E2E Morning Shift");

    // PUT /api/v1/shifts/{id} — update
    let req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/shifts/{}", shift_id),
        &token,
    )
    .set_json(json!({
        "name": "E2E Updated Shift",
        "is_active": false
    }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "update shift");
    let json = body_json(resp).await;
    assert_eq!(json["name"], "E2E Updated Shift");
    assert_eq!(json["is_active"], false);

    // Reactivate the shift so it can be used for assignments / attendance.
    let req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/shifts/{}", shift_id),
        &token,
    )
    .set_json(json!({ "is_active": true }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "reactivate shift");

    // POST /api/v1/shifts/assignments — create assignment
    let assign_start = chrono::Utc::now();
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/shifts/assignments",
        &token,
    )
    .set_json(json!({
        "shift_id": shift_id,
        "employee_id": employee_id,
        "start_date": assign_start.to_rfc3339(),
        "end_date": null
    }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED, "create assignment");
    let json = body_json(resp).await;
    let assignment_id = json["id"].as_i64().unwrap();
    assert_eq!(json["shift_id"], shift_id);
    assert_eq!(json["employee_id"], employee_id);

    // GET /api/v1/shifts/{shift_id}/assignments — list by shift
    let req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/shifts/{}/assignments", shift_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "list assignments by shift");
    let json = body_json(resp).await;
    let items = json.as_array().unwrap();
    assert!(items
        .iter()
        .any(|a| a["id"].as_i64() == Some(assignment_id)));

    // GET /api/v1/shifts/assignments/employee/{employee_id} — list by employee
    let req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/shifts/assignments/employee/{}", employee_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "list assignments by employee"
    );
    let json = body_json(resp).await;
    let items = json.as_array().unwrap();
    assert!(items
        .iter()
        .any(|a| a["id"].as_i64() == Some(assignment_id)));

    // POST /api/v1/shifts/attendance/clock-in — clock in
    let clock_in_ts = chrono::Utc::now();
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/shifts/attendance/clock-in",
        &token,
    )
    .set_json(json!({
        "employee_id": employee_id,
        "shift_id": shift_id,
        "timestamp": clock_in_ts.to_rfc3339(),
        "notes": "E2E clock in"
    }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED, "clock in");
    let json = body_json(resp).await;
    let attendance_id = json["id"].as_i64().unwrap();
    assert_eq!(json["employee_id"], employee_id);
    assert_eq!(json["shift_id"], shift_id);
    assert!(json["clock_in"].is_string());

    // POST /api/v1/shifts/attendance/clock-out — clock out
    let clock_out_ts = clock_in_ts + chrono::Duration::hours(8);
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/shifts/attendance/clock-out",
        &token,
    )
    .set_json(json!({
        "employee_id": employee_id,
        "timestamp": clock_out_ts.to_rfc3339(),
        "notes": "E2E clock out"
    }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "clock out");
    let json = body_json(resp).await;
    assert_eq!(json["id"], attendance_id);
    assert!(json["clock_out"].is_string());

    // GET /api/v1/shifts/attendance/employee/{employee_id} — list attendance
    let req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/shifts/attendance/employee/{}", employee_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "list shift attendance by employee"
    );
    let json = body_json(resp).await;
    let items = json.as_array().unwrap();
    assert!(items
        .iter()
        .any(|a| a["id"].as_i64() == Some(attendance_id)));

    // POST /api/v1/shifts/overtime — calculate overtime
    let ot_start = chrono::Utc::now() - chrono::Duration::days(7);
    let ot_end = chrono::Utc::now();
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/shifts/overtime",
        &token,
    )
    .set_json(json!({
        "employee_id": employee_id,
        "period_start": ot_start.to_rfc3339(),
        "period_end": ot_end.to_rfc3339(),
        "expected_hours_per_day": "8.00",
        "overtime_rate": "1.5"
    }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "calculate overtime");
    let json = body_json(resp).await;
    assert_eq!(json["employee_id"], employee_id);

    // POST /api/v1/shifts/reports — generate report
    let rep_start = chrono::Utc::now() - chrono::Duration::days(30);
    let rep_end = chrono::Utc::now();
    let req = auth_request(
        actix_web::http::Method::POST,
        "/api/v1/shifts/reports",
        &token,
    )
    .set_json(json!({
        "employee_id": employee_id,
        "period_start": rep_start.to_rfc3339(),
        "period_end": rep_end.to_rfc3339()
    }))
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "generate shift report");
    let json = body_json(resp).await;
    // The report endpoint returns a Vec<ShiftReport>; verify it is an array.
    assert!(json.is_array(), "shift report is an array");

    // GET /api/v1/shifts/deleted — should be empty
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/shifts/deleted",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "list deleted shifts (empty)");
    let json = body_json(resp).await;
    assert!(json.as_array().unwrap().is_empty());

    // DELETE /api/v1/shifts/assignments/{id} — remove assignment
    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/shifts/assignments/{}", assignment_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT, "delete assignment");

    // DELETE /api/v1/shifts/{id}/soft-delete — soft delete shift
    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/shifts/{}/soft-delete", shift_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT, "soft delete shift");

    // GET /api/v1/shifts/{id} — should be 404 after soft delete
    let req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/shifts/{}", shift_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::NOT_FOUND,
        "get shift after soft delete"
    );

    // GET /api/v1/shifts/deleted — should now contain the shift
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/shifts/deleted",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "list deleted shifts (populated)"
    );
    let json = body_json(resp).await;
    let items = json.as_array().unwrap();
    assert!(items.iter().any(|s| s["id"].as_i64() == Some(shift_id)));

    // PUT /api/v1/shifts/{id}/restore — restore
    let req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/shifts/{}/restore", shift_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK, "restore shift");
    let json = body_json(resp).await;
    assert_eq!(json["id"], shift_id);

    // Soft delete again then permanently destroy
    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/shifts/{}/soft-delete", shift_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::NO_CONTENT,
        "soft delete before destroy"
    );

    // DELETE /api/v1/shifts/{id}/destroy — permanent destroy
    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/shifts/{}/destroy", shift_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT, "destroy shift");

    // Restore should now fail (404)
    let req = auth_request(
        actix_web::http::Method::PUT,
        &format!("/api/v1/shifts/{}/restore", shift_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::NOT_FOUND,
        "restore after destroy"
    );
}

// ---------------------------------------------------------------------------
// Shift Planning: hard-delete (DELETE /api/v1/shifts/{id}) endpoint
// ---------------------------------------------------------------------------
// The resource-level DELETE maps to `delete_shift` (a hard delete in the
// in-memory repo). Verify it removes the shift without the soft-delete
// intermediate step.

#[actix_web::test]
async fn e2e_shifts_hard_delete_endpoint() {
    let app_state = create_test_app_state().await;
    let app = test::init_service(build_test_app_with_shift(&app_state)).await;
    let (token, _user_id) = register_admin(&app_state, 1).await;

    // Create a shift to hard-delete
    let req = auth_request(actix_web::http::Method::POST, "/api/v1/shifts", &token)
        .set_json(json!({
            "tenant_id": 1,
            "name": "E2E Hard Delete Shift",
            "shift_type": "Night",
            "start_time": "22:00:00",
            "end_time": "23:59:00",
            "break_duration_minutes": 45,
            "expected_hours": "7.50"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::CREATED,
        "create shift for hard delete"
    );
    let json = body_json(resp).await;
    let shift_id = json["id"].as_i64().unwrap();

    // DELETE /api/v1/shifts/{id} — hard delete
    let req = auth_request(
        actix_web::http::Method::DELETE,
        &format!("/api/v1/shifts/{}", shift_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::NO_CONTENT, "hard delete shift");

    // GET /api/v1/shifts/{id} — should be 404
    let req = auth_request(
        actix_web::http::Method::GET,
        &format!("/api/v1/shifts/{}", shift_id),
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::NOT_FOUND,
        "get after hard delete"
    );

    // Should not appear in the deleted list (hard delete, not soft delete)
    let req = auth_request(
        actix_web::http::Method::GET,
        "/api/v1/shifts/deleted",
        &token,
    )
    .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "deleted list after hard delete"
    );
    let json = body_json(resp).await;
    let items = json.as_array().unwrap();
    assert!(
        !items.iter().any(|s| s["id"].as_i64() == Some(shift_id)),
        "hard-deleted shift not in deleted list"
    );
}
