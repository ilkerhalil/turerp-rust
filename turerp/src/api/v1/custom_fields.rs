//! Custom fields API endpoints (v1)

use actix_web::{web, HttpResponse};

use crate::common::MessageResponse;
use crate::domain::custom_field::model::{
    CreateCustomFieldDefinition, CustomFieldModule, UpdateCustomFieldDefinition,
};
use crate::domain::custom_field::service::CustomFieldService;
use crate::error::{ApiError, ApiResult};
use crate::i18n::{resolve, I18n, Locale};
use crate::middleware::AdminUser;

/// Create a custom field definition (admin only)
#[utoipa::path(
    post,
    path = "/api/v1/custom-fields",
    tag = "Custom Fields",
    request_body = CreateCustomFieldDefinition,
    responses(
        (status = 201, description = "Custom field created"),
        (status = 400, description = "Validation error"),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "Forbidden - admin role required"),
        (status = 409, description = "Field name already exists in module")
    ),
    security(("bearer_auth" = []))
)]
pub async fn create_custom_field(
    admin_user: AdminUser,
    service: web::Data<CustomFieldService>,
    payload: web::Json<CreateCustomFieldDefinition>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let mut create = payload.into_inner();
    create.tenant_id = admin_user.0.tenant_id;
    match service.create(create).await {
        Ok(def) => Ok(HttpResponse::Created().json(def)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// List custom fields (optionally filtered by module)
#[utoipa::path(
    get,
    path = "/api/v1/custom-fields",
    tag = "Custom Fields",
    params(("module" = Option<String>, Query, description = "Filter by module")),
    responses(
        (status = 200, description = "List of custom field definitions"),
        (status = 401, description = "Not authenticated")
    ),
    security(("bearer_auth" = []))
)]
pub async fn list_custom_fields(
    admin_user: AdminUser,
    service: web::Data<CustomFieldService>,
    query: web::Query<ListQuery>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let tenant_id = admin_user.0.tenant_id;

    let result = match &query.module {
        Some(module_str) => match module_str.parse::<CustomFieldModule>() {
            Ok(module) => service.list_by_module(tenant_id, module).await,
            Err(e) => Err(ApiError::Validation(e)),
        },
        None => service.list_all(tenant_id).await,
    };

    match result {
        Ok(defs) => Ok(HttpResponse::Ok().json(defs)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get a custom field by ID
#[utoipa::path(
    get,
    path = "/api/v1/custom-fields/{id}",
    tag = "Custom Fields",
    params(("id" = i64, Path, description = "Custom field ID")),
    responses(
        (status = 200, description = "Custom field found"),
        (status = 401, description = "Not authenticated"),
        (status = 404, description = "Custom field not found")
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_custom_field(
    admin_user: AdminUser,
    service: web::Data<CustomFieldService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match service.get_by_id(*path, admin_user.0.tenant_id).await {
        Ok(def) => Ok(HttpResponse::Ok().json(def)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Update a custom field definition (admin only)
#[utoipa::path(
    put,
    path = "/api/v1/custom-fields/{id}",
    tag = "Custom Fields",
    params(("id" = i64, Path, description = "Custom field ID")),
    request_body = UpdateCustomFieldDefinition,
    responses(
        (status = 200, description = "Custom field updated"),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "Forbidden - admin role required"),
        (status = 404, description = "Custom field not found")
    ),
    security(("bearer_auth" = []))
)]
pub async fn update_custom_field(
    admin_user: AdminUser,
    service: web::Data<CustomFieldService>,
    path: web::Path<i64>,
    payload: web::Json<UpdateCustomFieldDefinition>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match service
        .update(*path, admin_user.0.tenant_id, payload.into_inner())
        .await
    {
        Ok(def) => Ok(HttpResponse::Ok().json(def)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Delete a custom field definition (admin only)
#[utoipa::path(
    delete,
    path = "/api/v1/custom-fields/{id}",
    tag = "Custom Fields",
    params(("id" = i64, Path, description = "Custom field ID")),
    responses(
        (status = 200, description = "Custom field deleted", body = MessageResponse),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "Forbidden - admin role required"),
        (status = 404, description = "Custom field not found")
    ),
    security(("bearer_auth" = []))
)]
pub async fn delete_custom_field(
    admin_user: AdminUser,
    service: web::Data<CustomFieldService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match service
        .soft_delete(*path, admin_user.0.tenant_id, admin_user.0.user_id()?)
        .await
    {
        Ok(()) => {
            let msg = i18n.t(locale.as_str(), "custom_field.deleted");
            Ok(HttpResponse::Ok().json(MessageResponse { message: msg }))
        }
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Restore a soft-deleted custom field definition (admin only)
#[utoipa::path(
    post,
    path = "/api/v1/custom-fields/{id}/restore",
    tag = "Custom Fields",
    params(("id" = i64, Path, description = "Custom field ID")),
    responses(
        (status = 200, description = "Custom field restored", body = MessageResponse),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "Forbidden - admin role required"),
        (status = 404, description = "Custom field not found")
    ),
    security(("bearer_auth" = []))
)]
pub async fn restore_custom_field(
    admin_user: AdminUser,
    service: web::Data<CustomFieldService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match service.restore(*path, admin_user.0.tenant_id).await {
        Ok(()) => {
            let msg = i18n.t(locale.as_str(), "custom_field.restored");
            Ok(HttpResponse::Ok().json(MessageResponse { message: msg }))
        }
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// List deleted custom field definitions (admin only)
#[utoipa::path(
    get,
    path = "/api/v1/custom-fields/deleted",
    tag = "Custom Fields",
    responses(
        (status = 200, description = "List of deleted custom fields"),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "Forbidden - admin role required")
    ),
    security(("bearer_auth" = []))
)]
pub async fn list_deleted_custom_fields(
    admin_user: AdminUser,
    service: web::Data<CustomFieldService>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match service.list_deleted(admin_user.0.tenant_id).await {
        Ok(defs) => Ok(HttpResponse::Ok().json(defs)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Permanently destroy a soft-deleted custom field definition (admin only)
#[utoipa::path(
    delete,
    path = "/api/v1/custom-fields/{id}/destroy",
    tag = "Custom Fields",
    params(("id" = i64, Path, description = "Custom field ID")),
    responses(
        (status = 204, description = "Custom field permanently destroyed"),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "Forbidden - admin role required"),
        (status = 404, description = "Custom field not found")
    ),
    security(("bearer_auth" = []))
)]
pub async fn destroy_custom_field(
    admin_user: AdminUser,
    service: web::Data<CustomFieldService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match service.destroy(*path, admin_user.0.tenant_id).await {
        Ok(()) => Ok(HttpResponse::NoContent().finish()),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub struct ListQuery {
    pub module: Option<String>,
}

/// Configure custom field routes for v1 API
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/v1/custom-fields")
            .route(web::get().to(list_custom_fields))
            .route(web::post().to(create_custom_field)),
    )
    .service(
        web::resource("/v1/custom-fields/{id}")
            .route(web::get().to(get_custom_field))
            .route(web::put().to(update_custom_field))
            .route(web::delete().to(delete_custom_field)),
    )
    .service(
        web::resource("/v1/custom-fields/{id}/restore").route(web::post().to(restore_custom_field)),
    )
    .service(
        web::resource("/v1/custom-fields/deleted").route(web::get().to(list_deleted_custom_fields)),
    )
    .service(
        web::resource("/v1/custom-fields/{id}/destroy")
            .route(web::delete().to(destroy_custom_field)),
    );
}
