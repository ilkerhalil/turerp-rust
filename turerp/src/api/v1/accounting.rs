//! Accounting API endpoints (v1)

use actix_web::{web, HttpResponse};

use crate::domain::accounting::model::{AccountType, CreateAccount, CreateJournalEntry};
use crate::domain::accounting::service::AccountingService;
use crate::error::ApiResult;
use crate::middleware::{AdminUser, AuthUser};

/// Create account (requires admin role)
#[utoipa::path(
    post, path = "/api/v1/accounting/accounts", tag = "Accounting",
    request_body = CreateAccount,
    responses((status = 201, description = "Account created"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn create_account(
    admin_user: AdminUser,
    accounting_service: web::Data<AccountingService>,
    payload: web::Json<CreateAccount>,
) -> ApiResult<HttpResponse> {
    let mut create = payload.into_inner();
    create.tenant_id = admin_user.0.tenant_id;
    let account = accounting_service.create_account(create).await?;
    Ok(HttpResponse::Created().json(account))
}

/// Get all accounts
#[utoipa::path(
    get, path = "/api/v1/accounting/accounts", tag = "Accounting",
    responses((status = 200, description = "List of accounts")),
    security(("bearer_auth" = []))
)]
pub async fn get_accounts(
    auth_user: AuthUser,
    accounting_service: web::Data<AccountingService>,
) -> ApiResult<HttpResponse> {
    let accounts = accounting_service
        .get_accounts_by_tenant(auth_user.0.tenant_id)
        .await?;
    Ok(HttpResponse::Ok().json(accounts))
}

/// Get accounts by type
#[utoipa::path(
    get, path = "/api/v1/accounting/accounts/type/{account_type}", tag = "Accounting",
    params(("account_type" = AccountType, Path, description = "Account type")),
    responses((status = 200, description = "List of accounts by type")),
    security(("bearer_auth" = []))
)]
pub async fn get_accounts_by_type(
    auth_user: AuthUser,
    accounting_service: web::Data<AccountingService>,
    path: web::Path<AccountType>,
) -> ApiResult<HttpResponse> {
    let accounts = accounting_service
        .get_accounts_by_type(auth_user.0.tenant_id, path.into_inner())
        .await?;
    Ok(HttpResponse::Ok().json(accounts))
}

/// Get account by ID
#[utoipa::path(
    get, path = "/api/v1/accounting/accounts/{id}", tag = "Accounting",
    params(("id" = i64, Path, description = "Account ID")),
    responses((status = 200, description = "Account found"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn get_account(
    _auth_user: AuthUser,
    accounting_service: web::Data<AccountingService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    let account = accounting_service.get_account(*path).await?;
    Ok(HttpResponse::Ok().json(account))
}

/// Create journal entry (requires admin role)
#[utoipa::path(
    post, path = "/api/v1/accounting/journal-entries", tag = "Accounting",
    request_body = CreateJournalEntry,
    responses((status = 201, description = "Journal entry created"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn create_journal_entry(
    admin_user: AdminUser,
    accounting_service: web::Data<AccountingService>,
    payload: web::Json<CreateJournalEntry>,
) -> ApiResult<HttpResponse> {
    let mut create = payload.into_inner();
    create.tenant_id = admin_user.0.tenant_id;
    create.created_by = admin_user.0.sub.parse().unwrap_or(0);
    let entry = accounting_service.create_journal_entry(create).await?;
    Ok(HttpResponse::Created().json(entry))
}

/// Get all journal entries
#[utoipa::path(
    get, path = "/api/v1/accounting/journal-entries", tag = "Accounting",
    responses((status = 200, description = "List of journal entries")),
    security(("bearer_auth" = []))
)]
pub async fn get_journal_entries(
    auth_user: AuthUser,
    accounting_service: web::Data<AccountingService>,
) -> ApiResult<HttpResponse> {
    let entries = accounting_service
        .get_journal_entries_by_tenant(auth_user.0.tenant_id)
        .await?;
    Ok(HttpResponse::Ok().json(entries))
}

/// Get journal entry by ID
#[utoipa::path(
    get, path = "/api/v1/accounting/journal-entries/{id}", tag = "Accounting",
    params(("id" = i64, Path, description = "Journal entry ID")),
    responses((status = 200, description = "Journal entry found"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn get_journal_entry(
    _auth_user: AuthUser,
    accounting_service: web::Data<AccountingService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    let entry = accounting_service.get_journal_entry(*path).await?;
    Ok(HttpResponse::Ok().json(entry))
}

/// Post journal entry (requires admin role)
#[utoipa::path(
    post, path = "/api/v1/accounting/journal-entries/{id}/post", tag = "Accounting",
    params(("id" = i64, Path, description = "Journal entry ID")),
    responses((status = 200, description = "Entry posted"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn post_journal_entry(
    _admin_user: AdminUser,
    accounting_service: web::Data<AccountingService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    let entry = accounting_service.post_journal_entry(*path).await?;
    Ok(HttpResponse::Ok().json(entry))
}

/// Void journal entry (requires admin role)
#[utoipa::path(
    post, path = "/api/v1/accounting/journal-entries/{id}/void", tag = "Accounting",
    params(("id" = i64, Path, description = "Journal entry ID")),
    responses((status = 200, description = "Entry voided"), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn void_journal_entry(
    _admin_user: AdminUser,
    accounting_service: web::Data<AccountingService>,
    path: web::Path<i64>,
) -> ApiResult<HttpResponse> {
    let entry = accounting_service.void_journal_entry(*path).await?;
    Ok(HttpResponse::Ok().json(entry))
}

/// Generate trial balance (requires authentication)
#[utoipa::path(
    post, path = "/api/v1/accounting/trial-balance", tag = "Accounting",
    request_body = TrialBalanceRequest,
    responses((status = 200, description = "Trial balance generated")),
    security(("bearer_auth" = []))
)]
pub async fn generate_trial_balance(
    auth_user: AuthUser,
    accounting_service: web::Data<AccountingService>,
    payload: web::Json<TrialBalanceRequest>,
) -> ApiResult<HttpResponse> {
    let balance = accounting_service
        .generate_trial_balance(
            auth_user.0.tenant_id,
            payload.period_start,
            payload.period_end,
        )
        .await?;
    Ok(HttpResponse::Ok().json(balance))
}

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub struct TrialBalanceRequest {
    pub period_start: chrono::DateTime<chrono::Utc>,
    pub period_end: chrono::DateTime<chrono::Utc>,
}

/// Configure accounting routes for v1 API
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/v1/accounting/accounts")
            .route(web::get().to(get_accounts))
            .route(web::post().to(create_account)),
    )
    .service(
        web::resource("/v1/accounting/accounts/type/{account_type}")
            .route(web::get().to(get_accounts_by_type)),
    )
    .service(web::resource("/v1/accounting/accounts/{id}").route(web::get().to(get_account)))
    .service(
        web::resource("/v1/accounting/journal-entries")
            .route(web::get().to(get_journal_entries))
            .route(web::post().to(create_journal_entry)),
    )
    .service(
        web::resource("/v1/accounting/journal-entries/{id}")
            .route(web::get().to(get_journal_entry)),
    )
    .service(
        web::resource("/v1/accounting/journal-entries/{id}/post")
            .route(web::post().to(post_journal_entry)),
    )
    .service(
        web::resource("/v1/accounting/journal-entries/{id}/void")
            .route(web::post().to(void_journal_entry)),
    )
    .service(
        web::resource("/v1/accounting/trial-balance").route(web::post().to(generate_trial_balance)),
    );
}
