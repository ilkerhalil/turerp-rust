//! Report generation API endpoints (v1)

use crate::common::{
    CreateJob, JobPriority, JobScheduler, JobType, ReportEngine, ReportFormat, ReportRequest,
    ReportType,
};
use crate::error::ApiError;
use crate::middleware::{AdminUser, AuthUser};
use actix_web::{web, HttpResponse};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Generate report request
#[derive(Debug, Deserialize, ToSchema)]
pub struct GenerateReportRequest {
    pub report_type: String,
    pub format: String,
    pub title: String,
    pub parameters: serde_json::Value,
    pub locale: Option<String>,
}

/// Queue report request
#[derive(Debug, Deserialize, ToSchema)]
pub struct QueueReportRequest {
    pub report_type: String,
    pub format: String,
    pub title: String,
    pub parameters: serde_json::Value,
    pub locale: Option<String>,
}

/// Report metadata response
#[derive(Debug, Serialize, ToSchema)]
pub struct ReportMetaResponse {
    pub id: i64,
    pub report_type: String,
    pub format: String,
    pub tenant_id: i64,
    pub title: String,
    pub filename: String,
    pub size_bytes: i64,
    pub generated_at: String,
    pub generated_by: Option<i64>,
}

/// Generate a report synchronously
#[utoipa::path(
    post,
    path = "/api/v1/reports/generate",
    tag = "Reports",
    request_body = GenerateReportRequest,
    responses(
        (status = 201, description = "Report generated"),
        (status = 400, description = "Invalid request"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn generate_report(
    admin_user: AdminUser,
    body: web::Json<GenerateReportRequest>,
    engine: web::Data<dyn ReportEngine>,
) -> Result<HttpResponse, ApiError> {
    let report_type = parse_report_type(&body.report_type)?;
    let format = parse_report_format(&body.format)?;
    let request = ReportRequest {
        report_type,
        format,
        tenant_id: admin_user.0.tenant_id,
        title: body.title.clone(),
        parameters: body.parameters.clone(),
        requested_by: Some(admin_user.0.user_id()?),
        locale: body.locale.clone(),
    };
    let report = engine.generate(request).await.map_err(ApiError::from)?;
    Ok(HttpResponse::Created()
        .content_type(report.content_type.clone())
        .insert_header((
            "Content-Disposition",
            format!("attachment; filename=\"{}\"", report.filename),
        ))
        .body(report.data))
}

/// Queue a report for async background generation
#[utoipa::path(
    post,
    path = "/api/v1/reports/queue",
    tag = "Reports",
    request_body = QueueReportRequest,
    responses(
        (status = 202, description = "Report queued"),
        (status = 400, description = "Invalid request"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn queue_report(
    admin_user: AdminUser,
    body: web::Json<QueueReportRequest>,
    scheduler: web::Data<dyn JobScheduler>,
) -> Result<HttpResponse, ApiError> {
    let _report_type = parse_report_type(&body.report_type)?;
    let _format = parse_report_format(&body.format)?;

    let params = serde_json::json!({
        "report_type": body.report_type,
        "format": body.format,
        "title": body.title,
        "parameters": body.parameters,
        "locale": body.locale,
        "tenant_id": admin_user.0.tenant_id,
        "requested_by": admin_user.0.user_id()?,    });

    let job = scheduler
        .schedule(
            CreateJob::new(
                JobType::GenerateReport {
                    tenant_id: admin_user.0.tenant_id,
                    report_type: body.report_type.clone(),
                    params: params.to_string(),
                },
                admin_user.0.tenant_id,
            )
            .with_priority(JobPriority::Normal),
        )
        .await
        .map_err(ApiError::Internal)?;

    Ok(HttpResponse::Accepted().json(serde_json::json!({
        "message": "Report queued for background generation",
        "job_id": job.id,
        "status": "pending"
    })))
}

/// Process the next pending report job
#[utoipa::path(
    post,
    path = "/api/v1/reports/process",
    tag = "Reports",
    responses(
        (status = 200, description = "Job processed"),
        (status = 204, description = "No pending jobs"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn process_report_job(
    _admin: AdminUser,
    scheduler: web::Data<dyn JobScheduler>,
    engine: web::Data<dyn ReportEngine>,
) -> Result<HttpResponse, ApiError> {
    let job = scheduler.next_pending().await.map_err(ApiError::Internal)?;

    let job = match job {
        Some(j) => j,
        None => return Ok(HttpResponse::NoContent().finish()),
    };

    let (tenant_id, report_type_str, params_str) = match &job.job_type {
        JobType::GenerateReport {
            tenant_id,
            report_type,
            params,
        } => (*tenant_id, report_type.clone(), params.clone()),
        _ => return Ok(HttpResponse::NoContent().finish()),
    };

    scheduler
        .mark_running(job.id)
        .await
        .map_err(ApiError::Internal)?;

    let params: serde_json::Value = serde_json::from_str(&params_str).unwrap_or_default();
    let report_type =
        parse_report_type(&report_type_str).unwrap_or(ReportType::Custom(report_type_str));
    let format = params
        .get("format")
        .and_then(|v| v.as_str())
        .unwrap_or("pdf");
    let format = parse_report_format(format).unwrap_or(ReportFormat::Pdf);
    let title = params
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("Report")
        .to_string();
    let parameters = params.get("parameters").cloned().unwrap_or_default();
    let requested_by = params.get("requested_by").and_then(|v| v.as_i64());
    let locale = params
        .get("locale")
        .and_then(|v| v.as_str())
        .map(String::from);

    let request = ReportRequest {
        report_type,
        format,
        tenant_id,
        title,
        parameters,
        requested_by,
        locale,
    };

    match engine.generate(request).await {
        Ok(report) => {
            engine.store_job_mapping(job.id, report.id).await.ok();
            scheduler
                .mark_completed(job.id)
                .await
                .map_err(ApiError::Internal)?;
            Ok(HttpResponse::Ok().json(serde_json::json!({
                "message": "Report generated",
                "job_id": job.id,
                "report_id": report.id
            })))
        }
        Err(e) => {
            scheduler
                .mark_failed(job.id, &e.to_string())
                .await
                .map_err(ApiError::Internal)?;
            Err(ApiError::Internal(format!(
                "Report generation failed: {}",
                e
            )))
        }
    }
}

/// List generated reports
#[utoipa::path(
    get,
    path = "/api/v1/reports",
    tag = "Reports",
    responses((status = 200, description = "List of report metadata")),
    security(("bearer_auth" = []))
)]
pub async fn list_reports(
    auth_user: AuthUser,
    engine: web::Data<dyn ReportEngine>,
) -> Result<HttpResponse, ApiError> {
    let reports = engine
        .list_reports(auth_user.0.tenant_id, 50, 0)
        .await
        .map_err(ApiError::from)?;
    let responses: Vec<ReportMetaResponse> = reports
        .iter()
        .map(|r| ReportMetaResponse {
            id: r.id,
            report_type: format!("{:?}", r.report_type),
            format: format!("{:?}", r.format),
            tenant_id: r.tenant_id,
            title: r.title.clone(),
            filename: r.filename.clone(),
            size_bytes: r.size_bytes,
            generated_at: r.generated_at.to_rfc3339(),
            generated_by: r.generated_by,
        })
        .collect();
    Ok(HttpResponse::Ok().json(responses))
}

/// Download a generated report
#[utoipa::path(
    get,
    path = "/api/v1/reports/{id}/download",
    tag = "Reports",
    params(("id" = i64, Path, description = "Report ID")),
    responses((status = 200, description = "Report file"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn download_report(
    auth_user: AuthUser,
    path: web::Path<i64>,
    engine: web::Data<dyn ReportEngine>,
) -> Result<HttpResponse, ApiError> {
    let id = path.into_inner();
    let report = engine
        .get_report(auth_user.0.tenant_id, id)
        .await
        .map_err(ApiError::from)?;
    match report {
        Some(r) => Ok(HttpResponse::Ok()
            .content_type(r.content_type.clone())
            .insert_header((
                "Content-Disposition",
                format!("attachment; filename=\"{}\"", r.filename),
            ))
            .body(r.data)),
        None => Err(ApiError::NotFound("Report not found".to_string())),
    }
}

/// Delete a generated report
#[utoipa::path(
    delete,
    path = "/api/v1/reports/{id}",
    tag = "Reports",
    params(("id" = i64, Path, description = "Report ID")),
    responses((status = 200, description = "Report deleted")),
    security(("bearer_auth" = []))
)]
pub async fn delete_report(
    admin_user: AdminUser,
    path: web::Path<i64>,
    engine: web::Data<dyn ReportEngine>,
) -> Result<HttpResponse, ApiError> {
    let id = path.into_inner();
    engine
        .delete_report(admin_user.0.tenant_id, id)
        .await
        .map_err(ApiError::from)?;
    Ok(HttpResponse::Ok().json(serde_json::json!({"message": "Report deleted"})))
}

/// Get report by job ID
#[utoipa::path(
    get,
    path = "/api/v1/reports/job/{job_id}",
    tag = "Reports",
    params(("job_id" = i64, Path, description = "Job ID")),
    responses((status = 200, description = "Report found"), (status = 404, description = "Not found")),
    security(("bearer_auth" = []))
)]
pub async fn get_report_by_job(
    auth_user: AuthUser,
    path: web::Path<i64>,
    engine: web::Data<dyn ReportEngine>,
) -> Result<HttpResponse, ApiError> {
    let job_id = path.into_inner();
    let report_id = engine
        .get_report_for_job(job_id)
        .await
        .map_err(ApiError::from)?;

    match report_id {
        Some(id) => {
            let report = engine
                .get_report(auth_user.0.tenant_id, id)
                .await
                .map_err(ApiError::from)?;
            match report {
                Some(r) => Ok(HttpResponse::Ok().json(serde_json::json!({
                    "report_id": r.id,
                    "filename": r.filename,
                    "format": format!("{:?}", r.format),
                    "generated_at": r.generated_at,
                }))),
                None => Err(ApiError::NotFound("Report not found".to_string())),
            }
        }
        None => Err(ApiError::NotFound(
            "No report found for this job".to_string(),
        )),
    }
}

fn parse_report_type(s: &str) -> Result<ReportType, ApiError> {
    Ok(match s {
        "invoice" => ReportType::Invoice,
        "trial_balance" => ReportType::TrialBalance,
        "balance_sheet" => ReportType::BalanceSheet,
        "income_statement" => ReportType::IncomeStatement,
        "payroll_summary" => ReportType::PayrollSummary,
        "stock_summary" => ReportType::StockSummary,
        "sales_report" => ReportType::SalesReport,
        "purchase_report" => ReportType::PurchaseReport,
        "aging_report" => ReportType::AgingReport,
        "edefter" => ReportType::EDefter,
        other => ReportType::Custom(other.to_string()),
    })
}

fn parse_report_format(s: &str) -> Result<ReportFormat, ApiError> {
    Ok(match s {
        "pdf" => ReportFormat::Pdf,
        "excel" => ReportFormat::Excel,
        "xml" => ReportFormat::Xml,
        "csv" => ReportFormat::Csv,
        "json" => ReportFormat::Json,
        _ => {
            return Err(ApiError::Validation(
                "Invalid format. Use: pdf, excel, xml, csv, json".to_string(),
            ))
        }
    })
}

/// Configure report routes
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/v1/reports")
            .route("/generate", web::post().to(generate_report))
            .route("/queue", web::post().to(queue_report))
            .route("/process", web::post().to(process_report_job))
            .route("/job/{job_id}", web::get().to(get_report_by_job))
            .route("", web::get().to(list_reports))
            .route("/{id}/download", web::get().to(download_report))
            .route("/{id}", web::delete().to(delete_report)),
    );
}
