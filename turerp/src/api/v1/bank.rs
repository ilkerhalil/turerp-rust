//! Bank integration API endpoints (v1)

use actix_web::{web, HttpResponse};

use crate::common::bank_parsers;
use crate::common::pagination::PaginationParams;
use crate::common::MessageResponse;
use crate::domain::bank::model::{
    CreateBankAccount, CreateReconciliationRule, ImportBankStatement, MatchTransaction,
    UpdateBankAccount, UpdateReconciliationRule,
};
use crate::domain::bank::service::BankService;
use crate::error::ApiResult;
use crate::i18n::{resolve, I18n, Locale};
use crate::middleware::{AdminUser, AuthUser};

/// Create a bank account (requires admin role)
#[utoipa::path(
    post,
    path = "/api/v1/bank/accounts",
    tag = "Bank",
    request_body = CreateBankAccount,
    responses(
        (status = 201, description = "Bank account created successfully"),
        (status = 400, description = "Validation error"),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "Forbidden - admin role required")
    ),
    security(("bearer_auth" = []))
)]
pub async fn create_account(
    admin_user: AdminUser,
    bank_service: web::Data<BankService>,
    payload: web::Json<CreateBankAccount>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let mut create = payload.into_inner();
    create.tenant_id = admin_user.0.tenant_id;
    match bank_service.create_account(create).await {
        Ok(account) => Ok(HttpResponse::Created().json(account)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get all bank accounts (requires authentication)
#[utoipa::path(
    get,
    path = "/api/v1/bank/accounts",
    tag = "Bank",
    responses(
        (status = 200, description = "List of bank accounts"),
        (status = 401, description = "Not authenticated")
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_accounts(
    auth_user: AuthUser,
    bank_service: web::Data<BankService>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match bank_service.get_accounts(auth_user.0.tenant_id).await {
        Ok(accounts) => Ok(HttpResponse::Ok().json(accounts)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get bank account by ID (requires authentication)
#[utoipa::path(
    get,
    path = "/api/v1/bank/accounts/{id}",
    tag = "Bank",
    params(("id" = i64, Path, description = "Bank account ID")),
    responses(
        (status = 200, description = "Bank account found"),
        (status = 401, description = "Not authenticated"),
        (status = 404, description = "Bank account not found")
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_account(
    auth_user: AuthUser,
    bank_service: web::Data<BankService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match bank_service.get_account(*path, auth_user.0.tenant_id).await {
        Ok(account) => Ok(HttpResponse::Ok().json(account)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Update a bank account (requires admin role)
#[utoipa::path(
    put,
    path = "/api/v1/bank/accounts/{id}",
    tag = "Bank",
    params(("id" = i64, Path, description = "Bank account ID")),
    request_body = UpdateBankAccount,
    responses(
        (status = 200, description = "Bank account updated"),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "Forbidden - admin role required"),
        (status = 404, description = "Bank account not found")
    ),
    security(("bearer_auth" = []))
)]
pub async fn update_account(
    admin_user: AdminUser,
    bank_service: web::Data<BankService>,
    path: web::Path<i64>,
    payload: web::Json<UpdateBankAccount>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match bank_service
        .update_account(*path, admin_user.0.tenant_id, payload.into_inner())
        .await
    {
        Ok(account) => Ok(HttpResponse::Ok().json(account)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Delete a bank account (requires admin role)
#[utoipa::path(
    delete,
    path = "/api/v1/bank/accounts/{id}",
    tag = "Bank",
    params(("id" = i64, Path, description = "Bank account ID")),
    responses(
        (status = 200, description = "Bank account deleted", body = MessageResponse),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "Forbidden - admin role required"),
        (status = 404, description = "Bank account not found")
    ),
    security(("bearer_auth" = []))
)]
pub async fn delete_account(
    admin_user: AdminUser,
    bank_service: web::Data<BankService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match bank_service
        .delete_account(*path, admin_user.0.tenant_id, admin_user.0.user_id()?)
        .await
    {
        Ok(()) => {
            let msg = i18n.t(locale.as_str(), "bank.deleted");
            Ok(HttpResponse::Ok().json(MessageResponse { message: msg }))
        }
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Restore a soft-deleted bank account (admin only)
#[utoipa::path(
    put,
    path = "/api/v1/bank/accounts/{id}/restore",
    tag = "Bank",
    params(("id" = i64, Path, description = "Bank account ID")),
    responses(
        (status = 200, description = "Bank account restored", body = BankAccountResponse),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "Forbidden - admin role required"),
        (status = 404, description = "Bank account not found or not deleted")
    ),
    security(("bearer_auth" = []))
)]
pub async fn restore_account(
    admin_user: AdminUser,
    bank_service: web::Data<BankService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    let account = bank_service
        .restore_account(*path, admin_user.0.tenant_id)
        .await?;
    Ok(HttpResponse::Ok().json(account))
}

/// Permanently delete a bank account (admin only)
#[utoipa::path(
    delete,
    path = "/api/v1/bank/accounts/{id}/destroy",
    tag = "Bank",
    params(("id" = i64, Path, description = "Bank account ID")),
    responses(
        (status = 204, description = "Bank account permanently deleted"),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "Forbidden - admin role required"),
        (status = 404, description = "Bank account not found")
    ),
    security(("bearer_auth" = []))
)]
pub async fn destroy_account(
    admin_user: AdminUser,
    bank_service: web::Data<BankService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    bank_service
        .destroy_account(*path, admin_user.0.tenant_id)
        .await?;
    Ok(HttpResponse::NoContent().finish())
}

/// Upload a statement file for a bank account (requires admin role)
#[utoipa::path(
    post,
    path = "/api/v1/bank/accounts/{id}/statements",
    tag = "Bank",
    params(("id" = i64, Path, description = "Bank account ID")),
    request_body = ImportBankStatement,
    responses(
        (status = 201, description = "Statement imported successfully"),
        (status = 400, description = "Invalid statement format"),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "Forbidden - admin role required"),
        (status = 404, description = "Bank account not found")
    ),
    security(("bearer_auth" = []))
)]
pub async fn upload_statement(
    admin_user: AdminUser,
    bank_service: web::Data<BankService>,
    path: web::Path<i64>,
    payload: web::Json<ImportBankStatement>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let account_id = *path;
    let import = payload.into_inner();

    // Parse statement data and create transactions
    let account = match bank_service
        .get_account(account_id, admin_user.0.tenant_id)
        .await
    {
        Ok(a) => a,
        Err(e) => return Ok(e.to_http_response(i18n, locale.as_str())),
    };

    let statement = match bank_service
        .import_statement(admin_user.0.tenant_id, account_id, import.clone())
        .await
    {
        Ok(s) => s,
        Err(e) => return Ok(e.to_http_response(i18n, locale.as_str())),
    };

    // Parse transactions from the raw data
    let parsed = match import.format {
        crate::domain::bank::model::StatementFormat::Mt940 => {
            bank_parsers::parse_mt940(&import.data)
        }
        crate::domain::bank::model::StatementFormat::Camt053 => {
            bank_parsers::parse_camt053(&import.data)
        }
        crate::domain::bank::model::StatementFormat::Xml => {
            let bank_code = account.bank_code;
            bank_parsers::parse_bank_xml(bank_code, &import.data)
        }
    };

    match bank_service
        .process_statement(admin_user.0.tenant_id, statement.id, parsed)
        .await
    {
        Ok(transactions) => Ok(HttpResponse::Created().json(serde_json::json!({
            "statement_id": statement.id,
            "transactions_imported": transactions.len(),
            "transactions": transactions.into_iter().map(crate::domain::bank::model::BankTransactionResponse::from).collect::<Vec<_>>()
        }))),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get transactions for a bank account (requires authentication)
#[utoipa::path(
    get,
    path = "/api/v1/bank/accounts/{id}/transactions",
    tag = "Bank",
    params(("id" = i64, Path, description = "Bank account ID"), PaginationParams),
    responses(
        (status = 200, description = "List of transactions"),
        (status = 401, description = "Not authenticated"),
        (status = 404, description = "Bank account not found")
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_transactions(
    auth_user: AuthUser,
    bank_service: web::Data<BankService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match bank_service
        .get_transactions(*path, auth_user.0.tenant_id)
        .await
    {
        Ok(transactions) => Ok(HttpResponse::Ok().json(transactions)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get unmatched transactions for a bank account (requires authentication)
#[utoipa::path(
    get,
    path = "/api/v1/bank/accounts/{id}/transactions/unmatched",
    tag = "Bank",
    params(("id" = i64, Path, description = "Bank account ID"), PaginationParams),
    responses(
        (status = 200, description = "List of unmatched transactions"),
        (status = 401, description = "Not authenticated"),
        (status = 404, description = "Bank account not found")
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_unmatched_transactions(
    auth_user: AuthUser,
    bank_service: web::Data<BankService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match bank_service
        .get_unmatched_transactions(*path, auth_user.0.tenant_id)
        .await
    {
        Ok(transactions) => Ok(HttpResponse::Ok().json(transactions)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Manually match a transaction (requires admin role)
#[utoipa::path(
    post,
    path = "/api/v1/bank/transactions/{id}/match",
    tag = "Bank",
    params(("id" = i64, Path, description = "Transaction ID")),
    request_body = MatchTransaction,
    responses(
        (status = 200, description = "Transaction matched"),
        (status = 400, description = "Already matched"),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "Forbidden - admin role required"),
        (status = 404, description = "Transaction not found")
    ),
    security(("bearer_auth" = []))
)]
pub async fn match_transaction(
    admin_user: AdminUser,
    bank_service: web::Data<BankService>,
    path: web::Path<i64>,
    payload: web::Json<MatchTransaction>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match bank_service
        .manual_match(*path, admin_user.0.tenant_id, payload.into_inner())
        .await
    {
        Ok(tx) => Ok(HttpResponse::Ok().json(tx)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Unmatch a transaction (requires admin role)
#[utoipa::path(
    post,
    path = "/api/v1/bank/transactions/{id}/unmatch",
    tag = "Bank",
    params(("id" = i64, Path, description = "Transaction ID")),
    responses(
        (status = 200, description = "Transaction unmatched"),
        (status = 400, description = "Already unmatched"),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "Forbidden - admin role required"),
        (status = 404, description = "Transaction not found")
    ),
    security(("bearer_auth" = []))
)]
pub async fn unmatch_transaction(
    admin_user: AdminUser,
    bank_service: web::Data<BankService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match bank_service
        .unmatch_transaction(*path, admin_user.0.tenant_id)
        .await
    {
        Ok(tx) => Ok(HttpResponse::Ok().json(tx)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get reconciliation report (requires authentication)
#[utoipa::path(
    get,
    path = "/api/v1/bank/reconciliation",
    tag = "Bank",
    responses(
        (status = 200, description = "Reconciliation report"),
        (status = 401, description = "Not authenticated")
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_reconciliation_report(
    auth_user: AuthUser,
    bank_service: web::Data<BankService>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match bank_service
        .get_reconciliation_report(auth_user.0.tenant_id)
        .await
    {
        Ok(report) => Ok(HttpResponse::Ok().json(report)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Trigger auto-reconciliation (requires admin role)
#[utoipa::path(
    post,
    path = "/api/v1/bank/reconcile",
    tag = "Bank",
    responses(
        (status = 200, description = "Auto-reconciliation completed"),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "Forbidden - admin role required")
    ),
    security(("bearer_auth" = []))
)]
pub async fn auto_reconcile(
    admin_user: AdminUser,
    bank_service: web::Data<BankService>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match bank_service.auto_reconcile(admin_user.0.tenant_id).await {
        Ok(report) => Ok(HttpResponse::Ok().json(report)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Create a reconciliation rule (requires admin role)
#[utoipa::path(
    post,
    path = "/api/v1/bank/rules",
    tag = "Bank",
    request_body = CreateReconciliationRule,
    responses(
        (status = 201, description = "Rule created successfully"),
        (status = 400, description = "Validation error"),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "Forbidden - admin role required")
    ),
    security(("bearer_auth" = []))
)]
pub async fn create_rule(
    admin_user: AdminUser,
    bank_service: web::Data<BankService>,
    payload: web::Json<CreateReconciliationRule>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let mut create = payload.into_inner();
    create.tenant_id = admin_user.0.tenant_id;
    match bank_service.create_rule(create).await {
        Ok(rule) => Ok(HttpResponse::Created().json(rule)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get all reconciliation rules (requires authentication)
#[utoipa::path(
    get,
    path = "/api/v1/bank/rules",
    tag = "Bank",
    responses(
        (status = 200, description = "List of reconciliation rules"),
        (status = 401, description = "Not authenticated")
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_rules(
    auth_user: AuthUser,
    bank_service: web::Data<BankService>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match bank_service.get_rules(auth_user.0.tenant_id).await {
        Ok(rules) => Ok(HttpResponse::Ok().json(rules)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get reconciliation rule by ID (requires authentication)
#[utoipa::path(
    get,
    path = "/api/v1/bank/rules/{id}",
    tag = "Bank",
    params(("id" = i64, Path, description = "Rule ID")),
    responses(
        (status = 200, description = "Rule found"),
        (status = 401, description = "Not authenticated"),
        (status = 404, description = "Rule not found")
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_rule(
    auth_user: AuthUser,
    bank_service: web::Data<BankService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match bank_service.get_rule(*path, auth_user.0.tenant_id).await {
        Ok(rule) => Ok(HttpResponse::Ok().json(rule)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Update a reconciliation rule (requires admin role)
#[utoipa::path(
    put,
    path = "/api/v1/bank/rules/{id}",
    tag = "Bank",
    params(("id" = i64, Path, description = "Rule ID")),
    request_body = UpdateReconciliationRule,
    responses(
        (status = 200, description = "Rule updated"),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "Forbidden - admin role required"),
        (status = 404, description = "Rule not found")
    ),
    security(("bearer_auth" = []))
)]
pub async fn update_rule(
    admin_user: AdminUser,
    bank_service: web::Data<BankService>,
    path: web::Path<i64>,
    payload: web::Json<UpdateReconciliationRule>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match bank_service
        .update_rule(*path, admin_user.0.tenant_id, payload.into_inner())
        .await
    {
        Ok(rule) => Ok(HttpResponse::Ok().json(rule)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Delete a reconciliation rule (requires admin role)
#[utoipa::path(
    delete,
    path = "/api/v1/bank/rules/{id}",
    tag = "Bank",
    params(("id" = i64, Path, description = "Rule ID")),
    responses(
        (status = 200, description = "Rule deleted", body = MessageResponse),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "Forbidden - admin role required"),
        (status = 404, description = "Rule not found")
    ),
    security(("bearer_auth" = []))
)]
pub async fn delete_rule(
    admin_user: AdminUser,
    bank_service: web::Data<BankService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match bank_service
        .delete_rule(*path, admin_user.0.tenant_id)
        .await
    {
        Ok(()) => {
            let msg = i18n.t(locale.as_str(), "bank.rule_deleted");
            Ok(HttpResponse::Ok().json(MessageResponse { message: msg }))
        }
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Configure bank routes for v1 API
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/v1/bank/accounts")
            .route(web::get().to(get_accounts))
            .route(web::post().to(create_account)),
    )
    .service(
        web::resource("/v1/bank/accounts/{id}")
            .route(web::get().to(get_account))
            .route(web::put().to(update_account))
            .route(web::delete().to(delete_account)),
    )
    .service(web::resource("/v1/bank/accounts/{id}/restore").route(web::put().to(restore_account)))
    .service(
        web::resource("/v1/bank/accounts/{id}/destroy").route(web::delete().to(destroy_account)),
    )
    .service(
        web::resource("/v1/bank/accounts/{id}/statements").route(web::post().to(upload_statement)),
    )
    .service(
        web::resource("/v1/bank/accounts/{id}/transactions").route(web::get().to(get_transactions)),
    )
    .service(
        web::resource("/v1/bank/accounts/{id}/transactions/unmatched")
            .route(web::get().to(get_unmatched_transactions)),
    )
    .service(
        web::resource("/v1/bank/transactions/{id}/match").route(web::post().to(match_transaction)),
    )
    .service(
        web::resource("/v1/bank/transactions/{id}/unmatch")
            .route(web::post().to(unmatch_transaction)),
    )
    .service(
        web::resource("/v1/bank/reconciliation").route(web::get().to(get_reconciliation_report)),
    )
    .service(web::resource("/v1/bank/reconcile").route(web::post().to(auto_reconcile)))
    .service(
        web::resource("/v1/bank/rules")
            .route(web::get().to(get_rules))
            .route(web::post().to(create_rule)),
    )
    .service(
        web::resource("/v1/bank/rules/{id}")
            .route(web::get().to(get_rule))
            .route(web::put().to(update_rule))
            .route(web::delete().to(delete_rule)),
    );
}
