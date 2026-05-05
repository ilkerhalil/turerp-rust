//! Full-text search API endpoints (v1)

use crate::common::{SearchDocument, SearchQuery, SearchService};
use crate::error::ApiError;
use crate::middleware::{AdminUser, AuthUser};
use actix_web::{web, HttpResponse};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Search request
#[derive(Debug, Deserialize, ToSchema)]
pub struct SearchRequest {
    pub query: String,
    pub entity_type: Option<String>,
    pub limit: Option<u32>,
    pub min_score: Option<f64>,
}

/// Search result response
#[derive(Debug, Serialize, ToSchema)]
pub struct SearchResultResponse {
    pub id: i64,
    pub entity_type: String,
    pub tenant_id: i64,
    pub title: String,
    pub description: String,
    pub score: f64,
}

/// Index document request
#[derive(Debug, Deserialize, ToSchema)]
pub struct IndexDocumentRequest {
    pub entity_type: String,
    pub entity_id: i64,
    pub title: String,
    pub description: Option<String>,
    pub searchable_text: String,
}

/// Search across entities
#[utoipa::path(
    get,
    path = "/api/v1/search",
    tag = "Search",
    params(
        ("q" = String, Query, description = "Search query"),
        ("entity_type" = Option<String>, Query, description = "Filter by entity type"),
        ("limit" = Option<u32>, Query, description = "Max results"),
    ),
    responses((status = 200, description = "Search results")),
    security(("bearer_auth" = []))
)]
pub async fn search(
    auth_user: AuthUser,
    query: web::Query<SearchRequest>,
    service: web::Data<dyn SearchService>,
) -> Result<HttpResponse, ApiError> {
    let mut sq = SearchQuery::new(query.query.clone(), auth_user.0.tenant_id);
    if let Some(entity_type) = &query.entity_type {
        sq = sq.with_entity_types(vec![entity_type.clone()]);
    }
    if let Some(limit) = query.limit {
        sq = sq.with_limit(limit);
    }
    if let Some(min_score) = query.min_score {
        sq.min_score = Some(min_score);
    }
    let results = service.search(sq).await?;
    let responses: Vec<SearchResultResponse> = results
        .iter()
        .map(|r| SearchResultResponse {
            id: r.id,
            entity_type: r.entity_type.clone(),
            tenant_id: r.tenant_id,
            title: r.title.clone(),
            description: r.description.clone().unwrap_or_default(),
            score: r.score,
        })
        .collect();
    Ok(HttpResponse::Ok().json(responses))
}

/// Index a document for search
#[utoipa::path(
    post,
    path = "/api/v1/search/index",
    tag = "Search",
    request_body = IndexDocumentRequest,
    responses((status = 201, description = "Document indexed")),
    security(("bearer_auth" = []))
)]
pub async fn index_document(
    admin_user: AdminUser,
    body: web::Json<IndexDocumentRequest>,
    service: web::Data<dyn SearchService>,
) -> Result<HttpResponse, ApiError> {
    let doc = SearchDocument {
        id: body.entity_id,
        entity_type: body.entity_type.clone(),
        tenant_id: admin_user.0.tenant_id,
        title: body.title.clone(),
        description: body.description.clone(),
        searchable_text: body.searchable_text.clone(),
    };
    service.index(&doc).await?;
    Ok(HttpResponse::Created().json(serde_json::json!({"message": "Document indexed"})))
}

/// Remove a document from search index
#[utoipa::path(
    delete,
    path = "/api/v1/search/{entity_type}/{id}",
    tag = "Search",
    params(
        ("entity_type" = String, Path, description = "Entity type"),
        ("id" = i64, Path, description = "Entity ID"),
    ),
    responses((status = 200, description = "Document removed")),
    security(("bearer_auth" = []))
)]
pub async fn remove_document(
    admin_user: AdminUser,
    path: web::Path<(String, i64)>,
    service: web::Data<dyn SearchService>,
) -> Result<HttpResponse, ApiError> {
    let (entity_type, id) = path.into_inner();
    service
        .remove(&entity_type, id, admin_user.0.tenant_id)
        .await?;
    Ok(HttpResponse::Ok().json(serde_json::json!({"message": "Document removed from index"})))
}

/// Reindex all documents for a tenant
#[utoipa::path(
    post,
    path = "/api/v1/search/reindex",
    tag = "Search",
    responses((status = 200, description = "Reindex started")),
    security(("bearer_auth" = []))
)]
pub async fn reindex(
    admin_user: AdminUser,
    service: web::Data<dyn SearchService>,
) -> Result<HttpResponse, ApiError> {
    service.reindex(admin_user.0.tenant_id).await?;
    Ok(HttpResponse::Ok().json(serde_json::json!({"message": "Reindex completed"})))
}

/// Configure search routes
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/v1/search")
            .route("", web::get().to(search))
            .route("/index", web::post().to(index_document))
            .route("/reindex", web::post().to(reindex))
            .route("/{entity_type}/{id}", web::delete().to(remove_document)),
    );
}
