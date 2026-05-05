//! Report generation API endpoints (v1)

use crate::common::{ReportEngine, ReportFormat, ReportRequest, ReportType};
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

/// Generate a report
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
    let report_type = match body.report_type.as_str() {
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
    };
    let format = match body.format.as_str() {
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
    };
    let request = ReportRequest {
        report_type,
        format,
        tenant_id: admin_user.0.tenant_id,
        title: body.title.clone(),
        parameters: body.parameters.clone(),
        requested_by: Some(admin_user.0.sub.parse::<i64>().unwrap_or(0)),
        locale: body.locale.clone(),
    };
    let report = engine.generate(request).await.map_err(ApiError::Internal)?;
    Ok(HttpResponse::Created()
        .content_type(report.content_type.clone())
        .insert_header((
            "Content-Disposition",
            format!("attachment; filename=\"{}\"", report.filename),
        ))
        .body(report.data))
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
        .map_err(ApiError::Internal)?;
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
        .map_err(ApiError::Internal)?;
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
        .map_err(ApiError::Internal)?;
    Ok(HttpResponse::Ok().json(serde_json::json!({"message": "Report deleted"})))
}

/// Configure report routes
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/v1/reports")
            .route("/generate", web::post().to(generate_report))
            .route("", web::get().to(list_reports))
            .route("/{id}/download", web::get().to(download_report))
            .route("/{id}", web::delete().to(delete_report)),
    );
}
