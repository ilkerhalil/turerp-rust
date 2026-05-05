//! API Key management endpoints

use crate::domain::api_key::model::{ApiKeyScope, CreateApiKey, UpdateApiKey};
use crate::domain::api_key::service::ApiKeyService;
use crate::error::ApiError;
use crate::middleware::api_key::ApiKeyAuth;
use crate::middleware::auth::AdminUser;
use actix_web::{web, HttpResponse};

/// Create a new API key (admin only)
#[utoipa::path(
    post,
    path = "/api/v1/api-keys",
    tag = "API Keys",
    request_body = CreateApiKey,
    responses(
        (status = 201, description = "API key created"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - admin only"),
    ),
    security(("bearer_auth" = [])),
)]
async fn create_api_key(
    _admin: AdminUser,
    service: web::Data<ApiKeyService>,
    body: web::Json<CreateApiKey>,
) -> Result<HttpResponse, ApiError> {
    let result = service.create_api_key(body.into_inner()).await?;
    Ok(HttpResponse::Created().json(result))
}

/// List API keys for a tenant (admin only)
#[utoipa::path(
    get,
    path = "/api/v1/api-keys/tenant/{tenant_id}",
    tag = "API Keys",
    params(
        ("tenant_id" = i64, Path, description = "Tenant ID"),
    ),
    responses(
        (status = 200, description = "List of API keys"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - admin only"),
    ),
    security(("bearer_auth" = [])),
)]
async fn list_api_keys(
    _admin: AdminUser,
    service: web::Data<ApiKeyService>,
    path: web::Path<i64>,
) -> Result<HttpResponse, ApiError> {
    let tenant_id = path.into_inner();
    let keys = service.list_api_keys(tenant_id).await?;
    Ok(HttpResponse::Ok().json(keys))
}

/// List API keys with pagination (admin only)
#[utoipa::path(
    get,
    path = "/api/v1/api-keys/tenant/{tenant_id}/paginated",
    tag = "API Keys",
    params(
        ("tenant_id" = i64, Path, description = "Tenant ID"),
        ("page" = Option<i32>, Query, description = "Page number"),
        ("per_page" = Option<i32>, Query, description = "Items per page"),
    ),
    responses(
        (status = 200, description = "Paginated list of API keys"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - admin only"),
    ),
    security(("bearer_auth" = [])),
)]
async fn list_api_keys_paginated(
    _admin: AdminUser,
    service: web::Data<ApiKeyService>,
    path: web::Path<i64>,
    query: web::Query<crate::common::pagination::PaginationParams>,
) -> Result<HttpResponse, ApiError> {
    let tenant_id = path.into_inner();
    let result = service
        .list_api_keys_paginated(tenant_id, query.page, query.per_page)
        .await?;
    Ok(HttpResponse::Ok().json(result))
}

/// Get a specific API key (admin only)
#[utoipa::path(
    get,
    path = "/api/v1/api-keys/{id}/tenant/{tenant_id}",
    tag = "API Keys",
    params(
        ("id" = i64, Path, description = "API key ID"),
        ("tenant_id" = i64, Path, description = "Tenant ID"),
    ),
    responses(
        (status = 200, description = "API key details"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - admin only"),
        (status = 404, description = "API key not found"),
    ),
    security(("bearer_auth" = [])),
)]
async fn get_api_key(
    _admin: AdminUser,
    service: web::Data<ApiKeyService>,
    path: web::Path<(i64, i64)>,
) -> Result<HttpResponse, ApiError> {
    let (id, tenant_id) = path.into_inner();
    let key = service.get_api_key(id, tenant_id).await?;
    Ok(HttpResponse::Ok().json(key))
}

/// Update an API key (admin only)
#[utoipa::path(
    put,
    path = "/api/v1/api-keys/{id}/tenant/{tenant_id}",
    tag = "API Keys",
    request_body = UpdateApiKey,
    params(
        ("id" = i64, Path, description = "API key ID"),
        ("tenant_id" = i64, Path, description = "Tenant ID"),
    ),
    responses(
        (status = 200, description = "API key updated"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - admin only"),
        (status = 404, description = "API key not found"),
    ),
    security(("bearer_auth" = [])),
)]
async fn update_api_key(
    _admin: AdminUser,
    service: web::Data<ApiKeyService>,
    path: web::Path<(i64, i64)>,
    body: web::Json<UpdateApiKey>,
) -> Result<HttpResponse, ApiError> {
    let (id, tenant_id) = path.into_inner();
    let key = service
        .update_api_key(id, tenant_id, body.into_inner())
        .await?;
    Ok(HttpResponse::Ok().json(key))
}

/// Delete an API key (admin only)
#[utoipa::path(
    delete,
    path = "/api/v1/api-keys/{id}/tenant/{tenant_id}",
    tag = "API Keys",
    params(
        ("id" = i64, Path, description = "API key ID"),
        ("tenant_id" = i64, Path, description = "Tenant ID"),
    ),
    responses(
        (status = 204, description = "API key deleted"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - admin only"),
        (status = 404, description = "API key not found"),
    ),
    security(("bearer_auth" = [])),
)]
async fn delete_api_key(
    _admin: AdminUser,
    service: web::Data<ApiKeyService>,
    path: web::Path<(i64, i64)>,
) -> Result<HttpResponse, ApiError> {
    let (id, tenant_id) = path.into_inner();
    service.delete_api_key(id, tenant_id).await?;
    Ok(HttpResponse::NoContent().finish())
}

/// Validate API key scope (for testing)
#[utoipa::path(
    get,
    path = "/api/v1/api-keys/check-scope/{scope}",
    tag = "API Keys",
    params(
        ("scope" = String, Path, description = "Scope to check"),
    ),
    responses(
        (status = 200, description = "Scope check result"),
        (status = 401, description = "Unauthorized - valid API key required"),
    ),
)]
async fn check_scope(
    api_key: ApiKeyAuth,
    _service: web::Data<ApiKeyService>,
    path: web::Path<String>,
) -> Result<HttpResponse, ApiError> {
    let scope_str = path.into_inner();
    let scope: ApiKeyScope = scope_str
        .parse()
        .map_err(|e: String| ApiError::BadRequest(e))?;
    let has_access = crate::middleware::api_key::has_scope(&api_key.0, &scope);
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "has_access": has_access,
        "scope": scope.to_string(),
        "api_key_id": api_key.0.api_key_id,
    })))
}

/// Configure API key routes
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/v1/api-keys")
            // Admin CRUD
            .route("", web::post().to(create_api_key))
            .route("/tenant/{tenant_id}", web::get().to(list_api_keys))
            .route(
                "/tenant/{tenant_id}/paginated",
                web::get().to(list_api_keys_paginated),
            )
            .route("/{id}/tenant/{tenant_id}", web::get().to(get_api_key))
            .route("/{id}/tenant/{tenant_id}", web::put().to(update_api_key))
            .route("/{id}/tenant/{tenant_id}", web::delete().to(delete_api_key))
            // Scope check (API key auth)
            .route("/check-scope/{scope}", web::get().to(check_scope)),
    );
}
