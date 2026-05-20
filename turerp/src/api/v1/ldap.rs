//! LDAP / Active Directory API endpoints (v1)

use actix_web::{web, HttpResponse};
use serde::Serialize;

use crate::common::MessageResponse;
use crate::domain::ldap::model::{CreateLdapConfig, TestLdapConnectionRequest, UpdateLdapConfig};
use crate::domain::ldap::service::LdapSyncService;
use crate::error::{ApiError, ApiResult};
use crate::i18n::{resolve, I18n, Locale};
use crate::json_resp;
use crate::middleware::AdminUser;

/// Create or update LDAP configuration (requires admin role)
#[utoipa::path(
    post,
    path = "/api/v1/ldap/config",
    tag = "LDAP",
    request_body = CreateLdapConfig,
    responses(
        (status = 201, description = "LDAP configuration created", body = LdapConfigResponse),
        (status = 400, description = "Validation error"),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "Forbidden - admin role required"),
        (status = 409, description = "Configuration already exists")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn create_ldap_config(
    admin_user: AdminUser,
    ldap_service: web::Data<LdapSyncService>,
    payload: web::Json<CreateLdapConfig>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let tenant_id = admin_user.0.tenant_id;
    json_resp!(
        ldap_service.create_ldap_config(tenant_id, payload.into_inner()),
        HttpResponse::Created,
        i18n,
        locale.as_str()
    )
}

/// Get LDAP configuration (requires authentication)
#[utoipa::path(
    get,
    path = "/api/v1/ldap/config",
    tag = "LDAP",
    responses(
        (status = 200, description = "LDAP configuration found", body = LdapConfigResponse),
        (status = 401, description = "Not authenticated"),
        (status = 404, description = "LDAP configuration not found")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn get_ldap_config(
    auth_user: crate::middleware::AuthUser,
    ldap_service: web::Data<LdapSyncService>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let tenant_id = auth_user.0.tenant_id;
    match ldap_service.get_ldap_config(tenant_id).await {
        Ok(Some(config)) => Ok(HttpResponse::Ok().json(config)),
        Ok(None) => {
            let err = ApiError::NotFound("LDAP configuration not found".to_string());
            Ok(err.to_http_response(i18n, locale.as_str()))
        }
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Update LDAP configuration (requires admin role)
#[utoipa::path(
    put,
    path = "/api/v1/ldap/config",
    tag = "LDAP",
    request_body = UpdateLdapConfig,
    responses(
        (status = 200, description = "LDAP configuration updated", body = LdapConfigResponse),
        (status = 400, description = "Validation error"),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "Forbidden - admin role required"),
        (status = 404, description = "LDAP configuration not found")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn update_ldap_config(
    admin_user: AdminUser,
    ldap_service: web::Data<LdapSyncService>,
    payload: web::Json<UpdateLdapConfig>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let tenant_id = admin_user.0.tenant_id;
    json_resp!(
        ldap_service.update_ldap_config(tenant_id, payload.into_inner()),
        HttpResponse::Ok,
        i18n,
        locale.as_str()
    )
}

/// Delete LDAP configuration (requires admin role)
#[utoipa::path(
    delete,
    path = "/api/v1/ldap/config",
    tag = "LDAP",
    responses(
        (status = 200, description = "LDAP configuration deleted", body = MessageResponse),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "Forbidden - admin role required"),
        (status = 404, description = "LDAP configuration not found")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn delete_ldap_config(
    admin_user: AdminUser,
    ldap_service: web::Data<LdapSyncService>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let tenant_id = admin_user.0.tenant_id;
    match ldap_service.delete_ldap_config(tenant_id).await {
        Ok(()) => {
            let msg = i18n.t(locale.as_str(), "ldap.deleted");
            Ok(HttpResponse::Ok().json(MessageResponse { message: msg }))
        }
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Test LDAP connection with stored configuration (requires admin role)
#[utoipa::path(
    post,
    path = "/api/v1/ldap/test",
    tag = "LDAP",
    request_body = TestLdapConnectionRequest,
    responses(
        (status = 200, description = "Connection test result", body = crate::api::v1::ldap::LdapConnectionTestResponse),
        (status = 400, description = "Validation error"),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "Forbidden - admin role required")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn test_ldap_connection(
    admin_user: AdminUser,
    ldap_service: web::Data<LdapSyncService>,
    payload: web::Json<TestLdapConnectionRequest>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let tenant_id = admin_user.0.tenant_id;

    // If request has explicit params, test with those; otherwise test stored config
    let request = payload.into_inner();
    let is_explicit = !request.ldap_url.is_empty();

    let result = if is_explicit {
        ldap_service.test_connection_with_params(request).await
    } else {
        ldap_service.test_connection(tenant_id).await
    };

    match result {
        Ok(success) => Ok(HttpResponse::Ok().json(LdapConnectionTestResponse {
            success,
            message: if success {
                i18n.t(locale.as_str(), "ldap.connection_ok")
            } else {
                i18n.t(locale.as_str(), "ldap.connection_failed")
            },
        })),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Trigger LDAP user synchronization (requires admin role)
#[utoipa::path(
    post,
    path = "/api/v1/ldap/sync",
    tag = "LDAP",
    responses(
        (status = 200, description = "Sync completed", body = LdapSyncResult),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "Forbidden - admin role required"),
        (status = 404, description = "LDAP configuration not found")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn sync_ldap_users(
    admin_user: AdminUser,
    ldap_service: web::Data<LdapSyncService>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let tenant_id = admin_user.0.tenant_id;
    json_resp!(
        ldap_service.sync_users(tenant_id),
        HttpResponse::Ok,
        i18n,
        locale.as_str()
    )
}

/// Response for LDAP connection test
#[derive(Serialize, utoipa::ToSchema)]
pub struct LdapConnectionTestResponse {
    pub success: bool,
    pub message: String,
}

/// Configure LDAP routes for v1 API
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/v1/ldap/config")
            .route(web::get().to(get_ldap_config))
            .route(web::post().to(create_ldap_config))
            .route(web::put().to(update_ldap_config))
            .route(web::delete().to(delete_ldap_config)),
    )
    .service(web::resource("/v1/ldap/test").route(web::post().to(test_ldap_connection)))
    .service(web::resource("/v1/ldap/sync").route(web::post().to(sync_ldap_users)));
}
