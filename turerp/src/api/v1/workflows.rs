//! Workflow API endpoints (v1)

use actix_web::{web, HttpResponse};
use serde::Deserialize;
use utoipa::ToSchema;

use crate::domain::workflow::model::{
    ApproveStep, CreateWorkflowTemplate, RejectStep, WorkflowInstanceResponse,
    WorkflowTemplateResponse,
};
use crate::domain::workflow::service::WorkflowService;
use crate::error::{ApiError, ApiResult};
use crate::i18n::{resolve, I18n, Locale};
use crate::middleware::{AdminUser, AuthUser};

/// Request body to start a workflow instance
#[derive(Debug, Deserialize, ToSchema)]
pub struct StartWorkflowRequest {
    pub template_id: i64,
    pub entity_id: i64,
    pub entity_type: String,
}

/// Create a workflow template (requires admin role)
#[utoipa::path(
    post, path = "/api/v1/workflows/templates", tag = "Workflows",
    request_body = CreateWorkflowTemplate,
    responses((status = 201, description = "Template created", body = WorkflowTemplateResponse), (status = 403, description = "Forbidden")),
    security(("bearer_auth" = []))
)]
pub async fn create_template(
    admin_user: AdminUser,
    workflow_service: web::Data<WorkflowService>,
    payload: web::Json<CreateWorkflowTemplate>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let create = payload.into_inner();
    match workflow_service
        .create_template(create, admin_user.0.tenant_id)
        .await
    {
        Ok(template) => Ok(HttpResponse::Created().json(WorkflowTemplateResponse::from(template))),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// List workflow templates
#[utoipa::path(
    get, path = "/api/v1/workflows/templates", tag = "Workflows",
    responses((status = 200, description = "List of templates", body = Vec<WorkflowTemplateResponse>)),
    security(("bearer_auth" = []))
)]
pub async fn list_templates(
    auth_user: AuthUser,
    workflow_service: web::Data<WorkflowService>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    match workflow_service.list_templates(auth_user.0.tenant_id).await {
        Ok(templates) => {
            let responses: Vec<WorkflowTemplateResponse> =
                templates.into_iter().map(Into::into).collect();
            Ok(HttpResponse::Ok().json(responses))
        }
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Start a workflow instance
#[utoipa::path(
    post, path = "/api/v1/workflows/instances", tag = "Workflows",
    request_body = StartWorkflowRequest,
    responses((status = 201, description = "Instance started", body = WorkflowInstanceResponse), (status = 400, description = "Bad request")),
    security(("bearer_auth" = []))
)]
pub async fn start_workflow(
    auth_user: AuthUser,
    workflow_service: web::Data<WorkflowService>,
    payload: web::Json<StartWorkflowRequest>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let req = payload.into_inner();
    let entity_type = match req.entity_type.parse() {
        Ok(et) => et,
        Err(e) => {
            return Ok(ApiError::Validation(e).to_http_response(i18n, locale.as_str()));
        }
    };
    let user_id = auth_user.0.user_id()?;
    match workflow_service
        .start_workflow(
            req.template_id,
            req.entity_id,
            entity_type,
            user_id,
            auth_user.0.tenant_id,
        )
        .await
    {
        Ok(instance) => Ok(HttpResponse::Created().json(WorkflowInstanceResponse::from(instance))),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Approve a workflow step
#[utoipa::path(
    post, path = "/api/v1/workflows/instances/{id}/approve", tag = "Workflows",
    params(("id" = i64, Path, description = "Instance ID")),
    request_body = ApproveStep,
    responses((status = 200, description = "Step approved", body = WorkflowInstanceResponse), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn approve_step(
    auth_user: AuthUser,
    workflow_service: web::Data<WorkflowService>,
    path: web::Path<i64>,
    payload: web::Json<ApproveStep>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let id = path.into_inner();
    let user_id = auth_user.0.user_id()?;
    // Find the current pending step for this instance
    let detail = match workflow_service
        .get_instance(id, auth_user.0.tenant_id)
        .await
    {
        Ok(d) => d,
        Err(e) => return Ok(e.to_http_response(i18n, locale.as_str())),
    };
    let step = match detail
        .steps
        .into_iter()
        .find(|s| s.status == crate::domain::workflow::model::WorkflowStepStatus::Pending)
    {
        Some(s) => s,
        None => {
            return Ok(ApiError::BadRequest("No pending step found".to_string())
                .to_http_response(i18n, locale.as_str()));
        }
    };
    match workflow_service
        .approve_step(
            id,
            step.id,
            user_id,
            payload.into_inner().comment,
            auth_user.0.tenant_id,
        )
        .await
    {
        Ok(instance) => Ok(HttpResponse::Ok().json(WorkflowInstanceResponse::from(instance))),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Reject a workflow step
#[utoipa::path(
    post, path = "/api/v1/workflows/instances/{id}/reject", tag = "Workflows",
    params(("id" = i64, Path, description = "Instance ID")),
    request_body = RejectStep,
    responses((status = 200, description = "Step rejected", body = WorkflowInstanceResponse), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn reject_step(
    auth_user: AuthUser,
    workflow_service: web::Data<WorkflowService>,
    path: web::Path<i64>,
    payload: web::Json<RejectStep>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let id = path.into_inner();
    let user_id = auth_user.0.user_id()?;
    let detail = match workflow_service
        .get_instance(id, auth_user.0.tenant_id)
        .await
    {
        Ok(d) => d,
        Err(e) => return Ok(e.to_http_response(i18n, locale.as_str())),
    };
    let step = match detail
        .steps
        .into_iter()
        .find(|s| s.status == crate::domain::workflow::model::WorkflowStepStatus::Pending)
    {
        Some(s) => s,
        None => {
            return Ok(ApiError::BadRequest("No pending step found".to_string())
                .to_http_response(i18n, locale.as_str()));
        }
    };
    match workflow_service
        .reject_step(
            id,
            step.id,
            user_id,
            payload.into_inner().comment,
            auth_user.0.tenant_id,
        )
        .await
    {
        Ok(instance) => Ok(HttpResponse::Ok().json(WorkflowInstanceResponse::from(instance))),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Resubmit a rejected workflow
#[utoipa::path(
    post, path = "/api/v1/workflows/instances/{id}/resubmit", tag = "Workflows",
    params(("id" = i64, Path, description = "Instance ID")),
    responses((status = 200, description = "Workflow resubmitted", body = WorkflowInstanceResponse), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn resubmit_workflow(
    auth_user: AuthUser,
    workflow_service: web::Data<WorkflowService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let id = path.into_inner();
    let user_id = auth_user.0.user_id()?;
    match workflow_service
        .resubmit(id, user_id, auth_user.0.tenant_id)
        .await
    {
        Ok(instance) => Ok(HttpResponse::Ok().json(WorkflowInstanceResponse::from(instance))),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get my pending approvals
#[utoipa::path(
    get, path = "/api/v1/workflows/pending", tag = "Workflows",
    responses((status = 200, description = "Pending approvals", body = Vec<WorkflowInstanceResponse>)),
    security(("bearer_auth" = []))
)]
pub async fn get_pending_approvals(
    auth_user: AuthUser,
    workflow_service: web::Data<WorkflowService>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let user_id = auth_user.0.user_id()?;
    match workflow_service
        .get_pending_approvals(user_id, auth_user.0.tenant_id)
        .await
    {
        Ok(instances) => {
            let responses: Vec<WorkflowInstanceResponse> =
                instances.into_iter().map(Into::into).collect();
            Ok(HttpResponse::Ok().json(responses))
        }
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get workflow instance details
#[utoipa::path(
    get, path = "/api/v1/workflows/instances/{id}", tag = "Workflows",
    params(("id" = i64, Path, description = "Instance ID")),
    responses((status = 200, description = "Instance details", body = WorkflowInstanceDetailResponse), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn get_instance(
    auth_user: AuthUser,
    workflow_service: web::Data<WorkflowService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let id = path.into_inner();
    match workflow_service
        .get_instance(id, auth_user.0.tenant_id)
        .await
    {
        Ok(detail) => Ok(HttpResponse::Ok().json(detail)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Get audit trail for a workflow instance
#[utoipa::path(
    get, path = "/api/v1/workflows/instances/{id}/audit", tag = "Workflows",
    params(("id" = i64, Path, description = "Instance ID")),
    responses((status = 200, description = "Audit trail", body = Vec<crate::domain::workflow::model::WorkflowAuditLogResponse>), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn get_instance_audit(
    auth_user: AuthUser,
    workflow_service: web::Data<WorkflowService>,
    path: web::Path<i64>,
    locale: Locale,
    i18n: Option<web::Data<I18n>>,
) -> ApiResult<HttpResponse> {
    let i18n = resolve(&i18n);
    let id = path.into_inner();
    match workflow_service
        .get_instance_audit_log(id, auth_user.0.tenant_id)
        .await
    {
        Ok(logs) => Ok(HttpResponse::Ok().json(logs)),
        Err(e) => Ok(e.to_http_response(i18n, locale.as_str())),
    }
}

/// Configure workflow routes for v1 API
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/v1/workflows/templates")
            .route(web::get().to(list_templates))
            .route(web::post().to(create_template)),
    )
    .service(web::resource("/v1/workflows/instances").route(web::post().to(start_workflow)))
    .service(web::resource("/v1/workflows/instances/{id}").route(web::get().to(get_instance)))
    .service(
        web::resource("/v1/workflows/instances/{id}/approve").route(web::post().to(approve_step)),
    )
    .service(
        web::resource("/v1/workflows/instances/{id}/reject").route(web::post().to(reject_step)),
    )
    .service(
        web::resource("/v1/workflows/instances/{id}/resubmit")
            .route(web::post().to(resubmit_workflow)),
    )
    .service(
        web::resource("/v1/workflows/instances/{id}/audit")
            .route(web::get().to(get_instance_audit)),
    )
    .service(web::resource("/v1/workflows/pending").route(web::get().to(get_pending_approvals)));
}
