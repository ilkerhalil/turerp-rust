//! Blockchain ledger API endpoints for e-Defter hash-chain compliance (v1)
//!
//! REST endpoints for cryptographic hash chain and Merkle tree operations
//! to ensure immutable audit trails for Turkish electronic ledger entries.

use actix_web::{web, HttpResponse};

use crate::domain::edefter::blockchain::model::HashChainResponse;
use crate::domain::edefter::blockchain::service::BlockchainLedgerService;
use crate::domain::edefter::service::EDefterService;
use crate::error::ApiResult;
use crate::i18n::{resolve, I18n, Locale};
use crate::middleware::{AdminUser, AuthUser};

/// Get the hash chain for a ledger period
#[utoipa::path(
    get, path = "/api/v1/edefter/periods/{id}/hash-chain", tag = "e-Defter Blockchain",
    params(("id" = i64, Path, description = "Ledger period ID")),
    responses((status = 200, description = "Hash chain entries", body = HashChainResponse), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn get_hash_chain(
    auth_user: AuthUser,
    blockchain_service: web::Data<BlockchainLedgerService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let id = path.into_inner();
    match blockchain_service
        .get_hash_chain(auth_user.0.tenant_id, id)
        .await
    {
        Ok(entries) => Ok(HttpResponse::Ok().json(HashChainResponse {
            period_id: id,
            count: entries.len(),
            entries,
        })),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Build and store the Merkle tree for a ledger period (requires admin)
#[utoipa::path(
    post, path = "/api/v1/edefter/periods/{id}/merkle-tree", tag = "e-Defter Blockchain",
    params(("id" = i64, Path, description = "Ledger period ID")),
    responses((status = 200, description = "Merkle tree built", body = MerkleTree), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn build_merkle_tree(
    admin_user: AdminUser,
    blockchain_service: web::Data<BlockchainLedgerService>,
    edefter_service: web::Data<EDefterService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let id = path.into_inner();
    let tenant_id = admin_user.0.tenant_id;

    // Fetch entries for the period
    let _entries = match edefter_service
        .find_entries_for_blockchain(id, tenant_id)
        .await
    {
        Ok(e) => e,
        Err(e) => return Ok(e.to_http_response(i18n, locale.as_str())),
    };

    // Get existing hash chain to extract entry hashes in order
    let chain = match blockchain_service.get_hash_chain(tenant_id, id).await {
        Ok(c) => c,
        Err(e) => return Ok(e.to_http_response(i18n, locale.as_str())),
    };

    if chain.is_empty() {
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "error": "No hash chain found for period; populate and build hash chain first"
        })));
    }

    let entry_hashes: Vec<String> = chain.iter().map(|c| c.entry_hash.clone()).collect();

    match blockchain_service
        .build_merkle_tree(tenant_id, id, entry_hashes)
        .await
    {
        Ok(tree) => Ok(HttpResponse::Ok().json(tree)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Verify the integrity of a ledger period (requires admin)
#[utoipa::path(
    post, path = "/api/v1/edefter/periods/{id}/verify", tag = "e-Defter Blockchain",
    params(("id" = i64, Path, description = "Ledger period ID")),
    responses((status = 200, description = "Verification result", body = VerifyResult), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn verify_period(
    admin_user: AdminUser,
    blockchain_service: web::Data<BlockchainLedgerService>,
    edefter_service: web::Data<EDefterService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let id = path.into_inner();
    let tenant_id = admin_user.0.tenant_id;

    let entries = match edefter_service
        .find_entries_for_blockchain(id, tenant_id)
        .await
    {
        Ok(e) => e,
        Err(e) => return Ok(e.to_http_response(i18n, locale.as_str())),
    };

    let merkle_tree = match blockchain_service.get_merkle_tree(tenant_id, id).await {
        Ok(t) => t,
        Err(e) => return Ok(e.to_http_response(i18n, locale.as_str())),
    };

    let expected_root = merkle_tree.map(|t| t.root_hash);

    match blockchain_service
        .verify_period_integrity(tenant_id, id, entries, expected_root)
        .await
    {
        Ok(result) => Ok(HttpResponse::Ok().json(result)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get the ledger hash state for a period
#[utoipa::path(
    get, path = "/api/v1/edefter/periods/{id}/hash-state", tag = "e-Defter Blockchain",
    params(("id" = i64, Path, description = "Ledger period ID")),
    responses((status = 200, description = "Hash state", body = LedgerHashState), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn get_hash_state(
    auth_user: AuthUser,
    blockchain_service: web::Data<BlockchainLedgerService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let id = path.into_inner();
    match blockchain_service
        .get_ledger_hash_state(auth_user.0.tenant_id, id)
        .await
    {
        Ok(Some(state)) => Ok(HttpResponse::Ok().json(state)),
        Ok(None) => Ok(HttpResponse::NotFound().json(serde_json::json!({
            "error": "Hash state not found for this period"
        }))),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Configure blockchain ledger routes for v1 API
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/v1/edefter/periods/{id}/hash-chain").route(web::get().to(get_hash_chain)),
    )
    .service(
        web::resource("/v1/edefter/periods/{id}/merkle-tree")
            .route(web::post().to(build_merkle_tree)),
    )
    .service(web::resource("/v1/edefter/periods/{id}/verify").route(web::post().to(verify_period)))
    .service(
        web::resource("/v1/edefter/periods/{id}/hash-state").route(web::get().to(get_hash_state)),
    );
}
