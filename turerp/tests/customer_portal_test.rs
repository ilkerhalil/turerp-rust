//! Customer Portal Integration Tests
//!
//! Run with: cargo test --test customer_portal_test

use actix_web::{body::to_bytes, http::StatusCode, test, web, App};
use serde_json::json;

use rust_decimal::Decimal;
use turerp::api::v1_customer_portal_configure;
use turerp::app::create_app_state_in_memory;
use turerp::config::Config;
use turerp::domain::cari::model::{CariType, CreateCari};
use turerp::domain::invoice::model::{
    CreateInvoice, CreateInvoiceLine, CreatePayment, InvoiceType,
};
use turerp::domain::sales::model::{CreateSalesOrder, CreateSalesOrderLine};
use turerp::middleware::JwtAuthMiddleware;
use turerp::utils::jwt::JwtService;

fn create_test_app_state() -> turerp::app::AppState {
    let config = Config::default();
    create_app_state_in_memory(&config).expect("app state creation failed")
}

fn build_test_app(
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
    let jwt = JwtService::new(
        Config::default().jwt.secret.clone(),
        Config::default().jwt.access_token_expiration,
        Config::default().jwt.refresh_token_expiration,
    );
    App::new()
        .wrap(JwtAuthMiddleware::new(jwt))
        .app_data(state.auth.auth_service.clone())
        .app_data(state.auth.user_service.clone())
        .app_data(state.auth.jwt_service.clone())
        .app_data(state.commerce.cari_service.clone())
        .app_data(state.commerce.stock_service.clone())
        .app_data(state.commerce.invoice_service.clone())
        .app_data(state.commerce.sales_service.clone())
        .app_data(state.integration.customer_portal_service.clone())
        .app_data(state.i18n.clone())
        .service(web::scope("/api").configure(v1_customer_portal_configure))
}

async fn create_customer_cari(state: &turerp::app::AppState, tenant_id: i64) -> i64 {
    let cari = state
        .commerce
        .cari_service
        .get_ref()
        .create_cari(CreateCari {
            code: format!("CUST{}", uuid::Uuid::new_v4()),
            name: "Test Customer".to_string(),
            cari_type: CariType::Customer,
            tax_number: Some("1234567890".to_string()),
            tax_office: Some("Test Office".to_string()),
            identity_number: None,
            email: Some("customer@test.com".to_string()),
            phone: Some("+90 555 123 4567".to_string()),
            address: Some("Test Address".to_string()),
            city: Some("Istanbul".to_string()),
            country: Some("Turkey".to_string()),
            postal_code: Some("34000".to_string()),
            credit_limit: Decimal::ZERO,
            default_currency: "TRY".to_string(),
            tenant_id,
            company_id: 1,
            created_by: 1,
        })
        .await
        .unwrap();
    cari.id
}

#[actix_web::test]
async fn test_portal_register_and_login() {
    let state = create_test_app_state();
    let app = test::init_service(build_test_app(&state)).await;

    let cari_id = create_customer_cari(&state, 1).await;

    // Register a portal user linked to this cari
    let req = test::TestRequest::post()
        .uri("/api/v1/customer-portal/register?tenant_id=1")
        .set_json(json!({
            "cari_id": cari_id,
            "email": "portaluser@test.com",
            "password": "PortalPass123!",
            "full_name": "Portal Test User",
            "phone": "+90 555 000 0000"
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let portal_user_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(portal_user_json["email"], "portaluser@test.com");
    assert_eq!(portal_user_json["cari_id"], cari_id);

    // Login with portal user
    let req = test::TestRequest::post()
        .uri("/api/v1/customer-portal/login?tenant_id=1")
        .set_json(json!({
            "email": "portaluser@test.com",
            "password": "PortalPass123!"
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let login_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let portal_token = login_json["access_token"].as_str().unwrap();
    assert!(!portal_token.is_empty());
    assert_eq!(login_json["token_type"], "Bearer");
}

#[actix_web::test]
async fn test_portal_login_invalid_credentials() {
    let state = create_test_app_state();
    let app = test::init_service(build_test_app(&state)).await;

    let req = test::TestRequest::post()
        .uri("/api/v1/customer-portal/login?tenant_id=1")
        .set_json(json!({
            "email": "nonexistent@test.com",
            "password": "WrongPass123!"
        }))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn test_portal_orders_requires_auth() {
    let state = create_test_app_state();
    let app = test::init_service(build_test_app(&state)).await;

    let req = test::TestRequest::get()
        .uri("/api/v1/customer-portal/orders")
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn test_portal_invoices_requires_auth() {
    let state = create_test_app_state();
    let app = test::init_service(build_test_app(&state)).await;

    let req = test::TestRequest::get()
        .uri("/api/v1/customer-portal/invoices")
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn test_portal_payments_requires_auth() {
    let state = create_test_app_state();
    let app = test::init_service(build_test_app(&state)).await;

    let req = test::TestRequest::get()
        .uri("/api/v1/customer-portal/payments")
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn test_portal_support_tickets_requires_auth() {
    let state = create_test_app_state();
    let app = test::init_service(build_test_app(&state)).await;

    let req = test::TestRequest::get()
        .uri("/api/v1/customer-portal/support-tickets")
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[actix_web::test]
async fn test_portal_support_tickets_crud() {
    let state = create_test_app_state();
    let app = test::init_service(build_test_app(&state)).await;

    let cari_id = create_customer_cari(&state, 1).await;

    // Register portal user
    let req = test::TestRequest::post()
        .uri("/api/v1/customer-portal/register?tenant_id=1")
        .set_json(json!({
            "cari_id": cari_id,
            "email": "ticketuser@test.com",
            "password": "PortalPass123!",
            "full_name": "Ticket User"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    // Login
    let req = test::TestRequest::post()
        .uri("/api/v1/customer-portal/login?tenant_id=1")
        .set_json(json!({
            "email": "ticketuser@test.com",
            "password": "PortalPass123!"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = to_bytes(resp.into_body()).await.unwrap();
    let login_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let portal_token = login_json["access_token"].as_str().unwrap();

    // Create a support ticket
    let req = test::TestRequest::post()
        .uri("/api/v1/customer-portal/support-tickets")
        .insert_header(("Authorization", format!("Bearer {}", portal_token)))
        .set_json(json!({
            "subject": "Test ticket",
            "description": "This is a test support ticket",
            "priority": "high",
            "category": "technical"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = to_bytes(resp.into_body()).await.unwrap();
    let ticket_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(ticket_json["subject"], "Test ticket");
    assert_eq!(ticket_json["status"], "open");

    // List support tickets
    let req = test::TestRequest::get()
        .uri("/api/v1/customer-portal/support-tickets")
        .insert_header(("Authorization", format!("Bearer {}", portal_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = to_bytes(resp.into_body()).await.unwrap();
    let tickets_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(tickets_json["data"].as_array().unwrap().len() >= 1);
}

async fn create_sales_order_for_cari(
    state: &turerp::app::AppState,
    tenant_id: i64,
    cari_id: i64,
) -> i64 {
    let order = state
        .commerce
        .sales_service
        .get_ref()
        .create_sales_order(CreateSalesOrder {
            tenant_id,
            company_id: 1,
            cari_id,
            order_date: chrono::Utc::now(),
            delivery_date: None,
            notes: None,
            shipping_address: None,
            billing_address: None,
            currency: "TRY".to_string(),
            exchange_rate: Decimal::ONE,
            lines: vec![CreateSalesOrderLine {
                product_id: None,
                description: "Test Product".to_string(),
                quantity: Decimal::new(10, 0),
                unit_price: Decimal::new(50, 0),
                tax_rate: Decimal::new(18, 0),
                discount_rate: Decimal::ZERO,
            }],
        })
        .await
        .unwrap();
    order.id
}

async fn create_invoice_for_cari(
    state: &turerp::app::AppState,
    tenant_id: i64,
    cari_id: i64,
) -> i64 {
    let now = chrono::Utc::now();
    let invoice = state
        .commerce
        .invoice_service
        .get_ref()
        .create_invoice(CreateInvoice {
            tenant_id,
            company_id: 1,
            invoice_type: InvoiceType::SalesInvoice,
            cari_id,
            issue_date: now,
            due_date: now + chrono::Duration::days(30),
            currency: "TRY".to_string(),
            exchange_rate: Decimal::ONE,
            notes: None,
            cost_center_id: None,
            lines: vec![CreateInvoiceLine {
                product_id: None,
                description: "Test Service".to_string(),
                quantity: Decimal::new(1, 0),
                unit_price: Decimal::new(100, 0),
                tax_rate: Decimal::new(18, 0),
                discount_rate: Decimal::ZERO,
            }],
        })
        .await
        .unwrap();
    invoice.id
}

async fn create_payment_for_invoice(
    state: &turerp::app::AppState,
    tenant_id: i64,
    invoice_id: i64,
) {
    state
        .commerce
        .invoice_service
        .get_ref()
        .create_payment(CreatePayment {
            tenant_id,
            company_id: 1,
            invoice_id,
            amount: Decimal::new(50, 0),
            currency: "TRY".to_string(),
            payment_date: chrono::Utc::now(),
            payment_method: "Bank Transfer".to_string(),
            reference_number: None,
            notes: None,
        })
        .await
        .unwrap();
}

#[actix_web::test]
async fn test_portal_order_history() {
    let state = create_test_app_state();
    let app = test::init_service(build_test_app(&state)).await;

    let cari_id = create_customer_cari(&state, 1).await;
    let order_id = create_sales_order_for_cari(&state, 1, cari_id).await;

    // Register portal user
    let req = test::TestRequest::post()
        .uri("/api/v1/customer-portal/register?tenant_id=1")
        .set_json(json!({
            "cari_id": cari_id,
            "email": "orderuser@test.com",
            "password": "PortalPass123!",
            "full_name": "Order User"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    // Login
    let req = test::TestRequest::post()
        .uri("/api/v1/customer-portal/login?tenant_id=1")
        .set_json(json!({
            "email": "orderuser@test.com",
            "password": "PortalPass123!"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = to_bytes(resp.into_body()).await.unwrap();
    let login_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let portal_token = login_json["access_token"].as_str().unwrap();

    // Get order history
    let req = test::TestRequest::get()
        .uri("/api/v1/customer-portal/orders")
        .insert_header(("Authorization", format!("Bearer {}", portal_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let orders = json["data"].as_array().unwrap();
    assert_eq!(orders.len(), 1);
    assert_eq!(orders[0]["id"], order_id);
}

#[actix_web::test]
async fn test_portal_invoice_list() {
    let state = create_test_app_state();
    let app = test::init_service(build_test_app(&state)).await;

    let cari_id = create_customer_cari(&state, 1).await;
    let invoice_id = create_invoice_for_cari(&state, 1, cari_id).await;

    // Register portal user
    let req = test::TestRequest::post()
        .uri("/api/v1/customer-portal/register?tenant_id=1")
        .set_json(json!({
            "cari_id": cari_id,
            "email": "invoiceuser@test.com",
            "password": "PortalPass123!",
            "full_name": "Invoice User"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    // Login
    let req = test::TestRequest::post()
        .uri("/api/v1/customer-portal/login?tenant_id=1")
        .set_json(json!({
            "email": "invoiceuser@test.com",
            "password": "PortalPass123!"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = to_bytes(resp.into_body()).await.unwrap();
    let login_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let portal_token = login_json["access_token"].as_str().unwrap();

    // Get invoices
    let req = test::TestRequest::get()
        .uri("/api/v1/customer-portal/invoices")
        .insert_header(("Authorization", format!("Bearer {}", portal_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let invoices = json["data"].as_array().unwrap();
    assert_eq!(invoices.len(), 1);
    assert_eq!(invoices[0]["id"], invoice_id);
}

#[actix_web::test]
async fn test_portal_payment_history() {
    let state = create_test_app_state();
    let app = test::init_service(build_test_app(&state)).await;

    let cari_id = create_customer_cari(&state, 1).await;
    let invoice_id = create_invoice_for_cari(&state, 1, cari_id).await;
    create_payment_for_invoice(&state, 1, invoice_id).await;

    // Register portal user
    let req = test::TestRequest::post()
        .uri("/api/v1/customer-portal/register?tenant_id=1")
        .set_json(json!({
            "cari_id": cari_id,
            "email": "paymentuser@test.com",
            "password": "PortalPass123!",
            "full_name": "Payment User"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    // Login
    let req = test::TestRequest::post()
        .uri("/api/v1/customer-portal/login?tenant_id=1")
        .set_json(json!({
            "email": "paymentuser@test.com",
            "password": "PortalPass123!"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = to_bytes(resp.into_body()).await.unwrap();
    let login_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let portal_token = login_json["access_token"].as_str().unwrap();

    // Get payment history
    let req = test::TestRequest::get()
        .uri("/api/v1/customer-portal/payments")
        .insert_header(("Authorization", format!("Bearer {}", portal_token)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let payments = json["data"].as_array().unwrap();
    assert_eq!(payments.len(), 1);
    assert_eq!(payments[0]["invoice_id"], invoice_id);
}

#[actix_web::test]
async fn test_portal_customer_isolation() {
    let state = create_test_app_state();
    let app = test::init_service(build_test_app(&state)).await;

    // Create two customers
    let cari_a_id = create_customer_cari(&state, 1).await;
    let cari_b_id = create_customer_cari(&state, 1).await;

    // Create data for both
    let order_a_id = create_sales_order_for_cari(&state, 1, cari_a_id).await;
    let invoice_b_id = create_invoice_for_cari(&state, 1, cari_b_id).await;

    // Register portal user A
    let req = test::TestRequest::post()
        .uri("/api/v1/customer-portal/register?tenant_id=1")
        .set_json(json!({
            "cari_id": cari_a_id,
            "email": "user_a@test.com",
            "password": "PortalPass123!",
            "full_name": "User A"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    // Login as A
    let req = test::TestRequest::post()
        .uri("/api/v1/customer-portal/login?tenant_id=1")
        .set_json(json!({
            "email": "user_a@test.com",
            "password": "PortalPass123!"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = to_bytes(resp.into_body()).await.unwrap();
    let login_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let token_a = login_json["access_token"].as_str().unwrap();

    // User A gets orders - should see only A's order
    let req = test::TestRequest::get()
        .uri("/api/v1/customer-portal/orders")
        .insert_header(("Authorization", format!("Bearer {}", token_a)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let orders = json["data"].as_array().unwrap();
    assert_eq!(orders.len(), 1);
    assert_eq!(orders[0]["id"], order_a_id);

    // User A gets invoices - should see none (no invoices for A)
    let req = test::TestRequest::get()
        .uri("/api/v1/customer-portal/invoices")
        .insert_header(("Authorization", format!("Bearer {}", token_a)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let invoices = json["data"].as_array().unwrap();
    assert!(invoices.is_empty(), "User A should not see B's invoices");

    // Register portal user B
    let req = test::TestRequest::post()
        .uri("/api/v1/customer-portal/register?tenant_id=1")
        .set_json(json!({
            "cari_id": cari_b_id,
            "email": "user_b@test.com",
            "password": "PortalPass123!",
            "full_name": "User B"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    // Login as B
    let req = test::TestRequest::post()
        .uri("/api/v1/customer-portal/login?tenant_id=1")
        .set_json(json!({
            "email": "user_b@test.com",
            "password": "PortalPass123!"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = to_bytes(resp.into_body()).await.unwrap();
    let login_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let token_b = login_json["access_token"].as_str().unwrap();

    // User B gets invoices - should see only B's invoice
    let req = test::TestRequest::get()
        .uri("/api/v1/customer-portal/invoices")
        .insert_header(("Authorization", format!("Bearer {}", token_b)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let invoices = json["data"].as_array().unwrap();
    assert_eq!(invoices.len(), 1);
    assert_eq!(invoices[0]["id"], invoice_b_id);

    // User B gets orders - should see none (no orders for B)
    let req = test::TestRequest::get()
        .uri("/api/v1/customer-portal/orders")
        .insert_header(("Authorization", format!("Bearer {}", token_b)))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let body = to_bytes(resp.into_body()).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let orders = json["data"].as_array().unwrap();
    assert!(orders.is_empty(), "User B should not see A's orders");
}

#[actix_web::test]
async fn test_portal_register_duplicate_email() {
    let state = create_test_app_state();
    let app = test::init_service(build_test_app(&state)).await;

    let cari_id = create_customer_cari(&state, 1).await;

    let req = test::TestRequest::post()
        .uri("/api/v1/customer-portal/register?tenant_id=1")
        .set_json(json!({
            "cari_id": cari_id,
            "email": "dup@test.com",
            "password": "PortalPass123!",
            "full_name": "Dup User"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CREATED);

    // Try to register again with same email
    let req = test::TestRequest::post()
        .uri("/api/v1/customer-portal/register?tenant_id=1")
        .set_json(json!({
            "cari_id": cari_id,
            "email": "dup@test.com",
            "password": "PortalPass123!",
            "full_name": "Dup User 2"
        }))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::CONFLICT);
}
