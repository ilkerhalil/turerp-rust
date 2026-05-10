//! Document Management System API endpoints (v1)

use actix_web::{web, HttpResponse};
use serde::Deserialize;

use crate::common::pagination::{default_page, default_per_page};
use crate::domain::document::model::{
    BulkRestoreRequest, CreateDocument, CreateDocumentCategory, CreateDocumentLink,
    CreateDocumentVersion, DocumentCategoryResponse, DocumentResponse, DocumentSearchParams,
    LinkedEntityType, UpdateDocument, UpdateDocumentCategory,
};
use crate::domain::document::service::DocumentService;
use crate::error::ApiResult;
use crate::i18n::{resolve, I18n, Locale};
use crate::middleware::{AdminUser, AuthUser};

/// Query parameters for searching documents
#[derive(Debug, Deserialize)]
pub struct SearchDocumentsQuery {
    #[serde(default = "default_page")]
    pub page: u32,
    #[serde(default = "default_per_page")]
    pub per_page: u32,
    pub query: Option<String>,
    pub category_id: Option<i64>,
    pub tags: Option<String>,
    pub entity_type: Option<String>,
    pub entity_id: Option<i64>,
    pub mime_type: Option<String>,
    pub uploaded_by: Option<i64>,
}

impl From<SearchDocumentsQuery> for DocumentSearchParams {
    fn from(q: SearchDocumentsQuery) -> Self {
        let tags = q.tags.map(|s| {
            s.split(',')
                .map(|t| t.trim().to_string())
                .filter(|t| !t.is_empty())
                .collect()
        });
        Self {
            query: q.query,
            category_id: q.category_id,
            tags,
            entity_type: q.entity_type,
            entity_id: q.entity_id,
            mime_type: q.mime_type,
            uploaded_by: q.uploaded_by,
            page: q.page,
            per_page: q.per_page,
        }
    }
}

/// Create a document metadata record
#[utoipa::path(
    post,
    path = "/api/v1/documents",
    tag = "Documents",
    request_body = CreateDocument,
    responses(
        (status = 201, description = "Document created", body = DocumentResponse),
        (status = 400, description = "Invalid input"),
        (status = 401, description = "Not authenticated")
    ),
    security(("bearer_auth" = []))
)]
pub async fn create_document(
    auth_user: AuthUser,
    doc_service: web::Data<DocumentService>,
    payload: web::Json<CreateDocument>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let mut create = payload.into_inner();
    create.tenant_id = auth_user.0.tenant_id;
    create.uploaded_by = Some(auth_user.0.user_id()?);
    match doc_service.create_document(create).await {
        Ok(doc) => Ok(HttpResponse::Created().json(DocumentResponse::from(doc))),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Search documents
#[utoipa::path(
    get,
    path = "/api/v1/documents",
    tag = "Documents",
    responses(
        (status = 200, description = "List of documents"),
        (status = 401, description = "Not authenticated")
    ),
    security(("bearer_auth" = []))
)]
pub async fn search_documents(
    auth_user: AuthUser,
    doc_service: web::Data<DocumentService>,
    query: web::Query<SearchDocumentsQuery>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let params = query.into_inner().into();
    match doc_service
        .search_documents(auth_user.0.tenant_id, params)
        .await
    {
        Ok(result) => Ok(HttpResponse::Ok().json(result)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get a document by ID
#[utoipa::path(
    get,
    path = "/api/v1/documents/{id}",
    tag = "Documents",
    params(("id" = i64, Path, description = "Document ID")),
    responses(
        (status = 200, description = "Document found", body = DocumentResponse),
        (status = 401, description = "Not authenticated"),
        (status = 404, description = "Document not found")
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_document(
    auth_user: AuthUser,
    doc_service: web::Data<DocumentService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let id = path.into_inner();
    match doc_service.get_document(id, auth_user.0.tenant_id).await {
        Ok(doc) => Ok(HttpResponse::Ok().json(DocumentResponse::from(doc))),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Update a document
#[utoipa::path(
    put,
    path = "/api/v1/documents/{id}",
    tag = "Documents",
    params(("id" = i64, Path, description = "Document ID")),
    request_body = UpdateDocument,
    responses(
        (status = 200, description = "Document updated", body = DocumentResponse),
        (status = 401, description = "Not authenticated"),
        (status = 404, description = "Document not found")
    ),
    security(("bearer_auth" = []))
)]
pub async fn update_document(
    auth_user: AuthUser,
    doc_service: web::Data<DocumentService>,
    path: web::Path<i64>,
    payload: web::Json<UpdateDocument>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let id = path.into_inner();
    match doc_service
        .update_document(id, auth_user.0.tenant_id, payload.into_inner())
        .await
    {
        Ok(doc) => Ok(HttpResponse::Ok().json(DocumentResponse::from(doc))),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Soft delete a document
#[utoipa::path(
    delete,
    path = "/api/v1/documents/{id}",
    tag = "Documents",
    params(("id" = i64, Path, description = "Document ID")),
    responses(
        (status = 204, description = "Document deleted"),
        (status = 401, description = "Not authenticated"),
        (status = 404, description = "Document not found")
    ),
    security(("bearer_auth" = []))
)]
pub async fn delete_document(
    auth_user: AuthUser,
    doc_service: web::Data<DocumentService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let id = path.into_inner();
    let deleted_by = auth_user.0.user_id()?;
    match doc_service
        .delete_document(id, auth_user.0.tenant_id, deleted_by)
        .await
    {
        Ok(()) => Ok(HttpResponse::NoContent().finish()),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Restore a soft-deleted document
#[utoipa::path(
    put,
    path = "/api/v1/documents/{id}/restore",
    tag = "Documents",
    params(("id" = i64, Path, description = "Document ID")),
    responses(
        (status = 200, description = "Document restored", body = DocumentResponse),
        (status = 401, description = "Not authenticated"),
        (status = 404, description = "Document not found")
    ),
    security(("bearer_auth" = []))
)]
pub async fn restore_document(
    auth_user: AuthUser,
    doc_service: web::Data<DocumentService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let id = path.into_inner();
    match doc_service
        .restore_document(id, auth_user.0.tenant_id)
        .await
    {
        Ok(doc) => Ok(HttpResponse::Ok().json(DocumentResponse::from(doc))),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// List soft-deleted documents
#[utoipa::path(
    get,
    path = "/api/v1/documents/deleted",
    tag = "Documents",
    responses(
        (status = 200, description = "List of deleted documents"),
        (status = 401, description = "Not authenticated")
    ),
    security(("bearer_auth" = []))
)]
pub async fn list_deleted_documents(
    auth_user: AuthUser,
    doc_service: web::Data<DocumentService>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match doc_service
        .list_deleted_documents(auth_user.0.tenant_id)
        .await
    {
        Ok(docs) => {
            let responses: Vec<DocumentResponse> =
                docs.into_iter().map(DocumentResponse::from).collect();
            Ok(HttpResponse::Ok().json(responses))
        }
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Permanently destroy a document (requires admin role)
#[utoipa::path(
    delete,
    path = "/api/v1/documents/{id}/destroy",
    tag = "Documents",
    params(("id" = i64, Path, description = "Document ID")),
    responses(
        (status = 204, description = "Document permanently deleted"),
        (status = 401, description = "Not authenticated"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Document not found")
    ),
    security(("bearer_auth" = []))
)]
pub async fn destroy_document(
    admin_user: AdminUser,
    doc_service: web::Data<DocumentService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let id = path.into_inner();
    match doc_service
        .destroy_document(id, admin_user.0.tenant_id)
        .await
    {
        Ok(()) => Ok(HttpResponse::NoContent().finish()),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Bulk restore soft-deleted documents
#[utoipa::path(
    post,
    path = "/api/v1/documents/bulk-restore",
    tag = "Documents",
    request_body = BulkRestoreRequest,
    responses(
        (status = 200, description = "Documents restored", body = BulkRestoreResponse<DocumentResponse>),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Not authenticated")
    ),
    security(("bearer_auth" = []))
)]
pub async fn bulk_restore_documents(
    auth_user: AuthUser,
    doc_service: web::Data<DocumentService>,
    payload: web::Json<BulkRestoreRequest>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let req = payload.into_inner();
    if req.ids.is_empty() {
        return Ok(
            crate::error::ApiError::BadRequest("IDs list cannot be empty".to_string())
                .to_http_response(i18n, locale.as_str()),
        );
    }
    if req.ids.len() > 100 {
        return Ok(crate::error::ApiError::BadRequest(
            "IDs list cannot exceed 100 items".to_string(),
        )
        .to_http_response(i18n, locale.as_str()));
    }
    match doc_service
        .bulk_restore_documents(req.ids, auth_user.0.tenant_id)
        .await
    {
        Ok(result) => Ok(HttpResponse::Ok().json(result)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

// --- Versions ---

/// Create a new version for a document
#[utoipa::path(
    post,
    path = "/api/v1/documents/{id}/versions",
    tag = "Documents",
    params(("id" = i64, Path, description = "Document ID")),
    request_body = CreateDocumentVersion,
    responses(
        (status = 201, description = "Version created"),
        (status = 401, description = "Not authenticated"),
        (status = 404, description = "Document not found")
    ),
    security(("bearer_auth" = []))
)]
pub async fn create_version(
    auth_user: AuthUser,
    doc_service: web::Data<DocumentService>,
    path: web::Path<i64>,
    payload: web::Json<CreateDocumentVersion>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let document_id = path.into_inner();
    let mut version = payload.into_inner();
    version.document_id = document_id;
    version.tenant_id = auth_user.0.tenant_id;
    version.created_by = Some(auth_user.0.user_id()?);
    match doc_service.create_version(version).await {
        Ok(v) => Ok(HttpResponse::Created().json(v)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// List versions for a document
#[utoipa::path(
    get,
    path = "/api/v1/documents/{id}/versions",
    tag = "Documents",
    params(("id" = i64, Path, description = "Document ID")),
    responses(
        (status = 200, description = "List of versions"),
        (status = 401, description = "Not authenticated"),
        (status = 404, description = "Document not found")
    ),
    security(("bearer_auth" = []))
)]
pub async fn list_versions(
    auth_user: AuthUser,
    doc_service: web::Data<DocumentService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let document_id = path.into_inner();
    match doc_service
        .list_versions(document_id, auth_user.0.tenant_id)
        .await
    {
        Ok(versions) => Ok(HttpResponse::Ok().json(versions)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get a specific version
#[utoipa::path(
    get,
    path = "/api/v1/documents/versions/{version_id}",
    tag = "Documents",
    params(("version_id" = i64, Path, description = "Version ID")),
    responses(
        (status = 200, description = "Version found"),
        (status = 401, description = "Not authenticated"),
        (status = 404, description = "Version not found")
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_version(
    auth_user: AuthUser,
    doc_service: web::Data<DocumentService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let version_id = path.into_inner();
    match doc_service
        .get_version(version_id, auth_user.0.tenant_id)
        .await
    {
        Ok(v) => Ok(HttpResponse::Ok().json(v)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

// --- Categories ---

/// Create a document category
#[utoipa::path(
    post,
    path = "/api/v1/documents/categories",
    tag = "Documents",
    request_body = CreateDocumentCategory,
    responses(
        (status = 201, description = "Category created", body = DocumentCategoryResponse),
        (status = 400, description = "Invalid input"),
        (status = 401, description = "Not authenticated")
    ),
    security(("bearer_auth" = []))
)]
pub async fn create_category(
    auth_user: AuthUser,
    doc_service: web::Data<DocumentService>,
    payload: web::Json<CreateDocumentCategory>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let mut category = payload.into_inner();
    category.tenant_id = auth_user.0.tenant_id;
    match doc_service.create_category(category).await {
        Ok(cat) => Ok(HttpResponse::Created().json(DocumentCategoryResponse::from(cat))),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// List document categories
#[utoipa::path(
    get,
    path = "/api/v1/documents/categories",
    tag = "Documents",
    responses(
        (status = 200, description = "List of categories"),
        (status = 401, description = "Not authenticated")
    ),
    security(("bearer_auth" = []))
)]
pub async fn list_categories(
    auth_user: AuthUser,
    doc_service: web::Data<DocumentService>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match doc_service.list_categories(auth_user.0.tenant_id).await {
        Ok(cats) => {
            let responses: Vec<DocumentCategoryResponse> = cats
                .into_iter()
                .map(DocumentCategoryResponse::from)
                .collect();
            Ok(HttpResponse::Ok().json(responses))
        }
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get a category by ID
#[utoipa::path(
    get,
    path = "/api/v1/documents/categories/{id}",
    tag = "Documents",
    params(("id" = i64, Path, description = "Category ID")),
    responses(
        (status = 200, description = "Category found", body = DocumentCategoryResponse),
        (status = 401, description = "Not authenticated"),
        (status = 404, description = "Category not found")
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_category(
    auth_user: AuthUser,
    doc_service: web::Data<DocumentService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let id = path.into_inner();
    match doc_service.get_category(id, auth_user.0.tenant_id).await {
        Ok(cat) => Ok(HttpResponse::Ok().json(DocumentCategoryResponse::from(cat))),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Update a category
#[utoipa::path(
    put,
    path = "/api/v1/documents/categories/{id}",
    tag = "Documents",
    params(("id" = i64, Path, description = "Category ID")),
    request_body = UpdateDocumentCategory,
    responses(
        (status = 200, description = "Category updated", body = DocumentCategoryResponse),
        (status = 401, description = "Not authenticated"),
        (status = 404, description = "Category not found")
    ),
    security(("bearer_auth" = []))
)]
pub async fn update_category(
    auth_user: AuthUser,
    doc_service: web::Data<DocumentService>,
    path: web::Path<i64>,
    payload: web::Json<UpdateDocumentCategory>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let id = path.into_inner();
    match doc_service
        .update_category(id, auth_user.0.tenant_id, payload.into_inner())
        .await
    {
        Ok(cat) => Ok(HttpResponse::Ok().json(DocumentCategoryResponse::from(cat))),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Delete a category
#[utoipa::path(
    delete,
    path = "/api/v1/documents/categories/{id}",
    tag = "Documents",
    params(("id" = i64, Path, description = "Category ID")),
    responses(
        (status = 204, description = "Category deleted"),
        (status = 401, description = "Not authenticated"),
        (status = 404, description = "Category not found")
    ),
    security(("bearer_auth" = []))
)]
pub async fn delete_category(
    auth_user: AuthUser,
    doc_service: web::Data<DocumentService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let id = path.into_inner();
    match doc_service.delete_category(id, auth_user.0.tenant_id).await {
        Ok(()) => Ok(HttpResponse::NoContent().finish()),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

// --- Entity Links ---

/// Link a document to an entity
#[utoipa::path(
    post,
    path = "/api/v1/documents/{id}/links",
    tag = "Documents",
    params(("id" = i64, Path, description = "Document ID")),
    request_body = CreateDocumentLink,
    responses(
        (status = 201, description = "Link created"),
        (status = 400, description = "Invalid input"),
        (status = 401, description = "Not authenticated"),
        (status = 404, description = "Document not found")
    ),
    security(("bearer_auth" = []))
)]
pub async fn create_link(
    auth_user: AuthUser,
    doc_service: web::Data<DocumentService>,
    path: web::Path<i64>,
    payload: web::Json<CreateDocumentLink>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let document_id = path.into_inner();
    let mut link = payload.into_inner();
    link.document_id = document_id;
    link.tenant_id = auth_user.0.tenant_id;
    match doc_service.link_document(link).await {
        Ok(l) => Ok(HttpResponse::Created().json(l)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// List links for a document
#[utoipa::path(
    get,
    path = "/api/v1/documents/{id}/links",
    tag = "Documents",
    params(("id" = i64, Path, description = "Document ID")),
    responses(
        (status = 200, description = "List of links"),
        (status = 401, description = "Not authenticated"),
        (status = 404, description = "Document not found")
    ),
    security(("bearer_auth" = []))
)]
pub async fn list_document_links(
    auth_user: AuthUser,
    doc_service: web::Data<DocumentService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let document_id = path.into_inner();
    match doc_service
        .list_document_links(document_id, auth_user.0.tenant_id)
        .await
    {
        Ok(links) => Ok(HttpResponse::Ok().json(links)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Find documents linked to an entity
#[utoipa::path(
    get,
    path = "/api/v1/documents/by-entity/{entity_type}/{entity_id}",
    tag = "Documents",
    params(
        ("entity_type" = String, Path, description = "Entity type (invoice, order, cari, product, project, employee, purchase_order, sales_order, work_order, other)"),
        ("entity_id" = i64, Path, description = "Entity ID")
    ),
    responses(
        (status = 200, description = "List of documents"),
        (status = 401, description = "Not authenticated")
    ),
    security(("bearer_auth" = []))
)]
pub async fn find_by_entity(
    auth_user: AuthUser,
    doc_service: web::Data<DocumentService>,
    path: web::Path<(String, i64)>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let (entity_type_str, entity_id) = path.into_inner();
    let entity_type = match entity_type_str.parse::<LinkedEntityType>() {
        Ok(et) => et,
        Err(e) => {
            return Ok(
                crate::error::ApiError::Validation(e).to_http_response(i18n, locale.as_str())
            );
        }
    };
    match doc_service
        .find_documents_by_entity(auth_user.0.tenant_id, entity_type, entity_id)
        .await
    {
        Ok(docs) => {
            let responses: Vec<DocumentResponse> =
                docs.into_iter().map(DocumentResponse::from).collect();
            Ok(HttpResponse::Ok().json(responses))
        }
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Configure document management routes for v1 API
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/v1/documents")
            .route(web::get().to(search_documents))
            .route(web::post().to(create_document)),
    )
    .service(
        web::resource("/v1/documents/bulk-restore").route(web::post().to(bulk_restore_documents)),
    )
    .service(web::resource("/v1/documents/deleted").route(web::get().to(list_deleted_documents)))
    .service(
        web::resource("/v1/documents/{id}")
            .route(web::get().to(get_document))
            .route(web::put().to(update_document))
            .route(web::delete().to(delete_document)),
    )
    .service(web::resource("/v1/documents/{id}/restore").route(web::put().to(restore_document)))
    .service(web::resource("/v1/documents/{id}/destroy").route(web::delete().to(destroy_document)))
    .service(
        web::resource("/v1/documents/{id}/versions")
            .route(web::get().to(list_versions))
            .route(web::post().to(create_version)),
    )
    .service(web::resource("/v1/documents/versions/{version_id}").route(web::get().to(get_version)))
    .service(
        web::resource("/v1/documents/categories")
            .route(web::get().to(list_categories))
            .route(web::post().to(create_category)),
    )
    .service(
        web::resource("/v1/documents/categories/{id}")
            .route(web::get().to(get_category))
            .route(web::put().to(update_category))
            .route(web::delete().to(delete_category)),
    )
    .service(
        web::resource("/v1/documents/{id}/links")
            .route(web::get().to(list_document_links))
            .route(web::post().to(create_link)),
    )
    .service(
        web::resource("/v1/documents/by-entity/{entity_type}/{entity_id}")
            .route(web::get().to(find_by_entity)),
    );
}
