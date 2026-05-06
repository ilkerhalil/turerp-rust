//! Background job scheduler API endpoints

use crate::common::{
    CreateJob, Job, JobPriority, JobScheduler, JobStatus, JobType, PaginationParams,
};
use crate::error::ApiError;
use crate::middleware::AdminUser;
use actix_web::{web, HttpResponse};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Job response DTO
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct JobResponse {
    pub id: i64,
    pub job_type: String,
    pub status: String,
    pub priority: String,
    pub tenant_id: i64,
    pub attempts: u32,
    pub max_attempts: u32,
    pub scheduled_at: Option<DateTime<Utc>>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl JobResponse {
    fn from_job(job: &Job) -> Self {
        Self {
            id: job.id,
            job_type: match &job.job_type {
                JobType::CalculateDepreciation { .. } => "calculate_depreciation".to_string(),
                JobType::RunPayroll { .. } => "run_payroll".to_string(),
                JobType::SendReminders { .. } => "send_reminders".to_string(),
                JobType::ArchiveLogs { .. } => "archive_logs".to_string(),
                JobType::GenerateReport { .. } => "generate_report".to_string(),
                JobType::SendNotification { .. } => "send_notification".to_string(),
                JobType::Custom { .. } => "custom".to_string(),
            },
            status: match job.status {
                JobStatus::Pending => "pending".to_string(),
                JobStatus::Running => "running".to_string(),
                JobStatus::Completed => "completed".to_string(),
                JobStatus::Failed => "failed".to_string(),
                JobStatus::Cancelled => "cancelled".to_string(),
                JobStatus::Scheduled => "scheduled".to_string(),
            },
            priority: match job.priority {
                JobPriority::Low => "low".to_string(),
                JobPriority::Normal => "normal".to_string(),
                JobPriority::High => "high".to_string(),
                JobPriority::Critical => "critical".to_string(),
            },
            tenant_id: job.tenant_id,
            attempts: job.attempts,
            max_attempts: job.max_attempts,
            scheduled_at: job.scheduled_at,
            started_at: job.started_at,
            completed_at: job.completed_at,
            last_error: job.last_error.clone(),
            created_at: job.created_at,
        }
    }
}

/// Create job request
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateJobRequest {
    pub job_type: String,
    pub tenant_id: i64,
    pub priority: Option<String>,
    pub max_attempts: Option<u32>,
    pub scheduled_at: Option<DateTime<Utc>>,
    pub asset_id: Option<i64>,
    pub period: Option<String>,
    pub older_than_days: Option<i32>,
    pub report_type: Option<String>,
    pub params: Option<String>,
    pub custom_name: Option<String>,
    pub custom_payload: Option<String>,
}

/// Fail job request
#[derive(Debug, Deserialize, ToSchema)]
pub struct FailJobRequest {
    pub error: String,
}

/// Schedule a new job
#[utoipa::path(
    post,
    path = "/api/v1/jobs",
    tag = "Jobs",
    request_body = CreateJobRequest,
    responses(
        (status = 201, description = "Job scheduled successfully", body = JobResponse),
        (status = 400, description = "Invalid request"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn schedule_job(
    _admin: AdminUser,
    body: web::Json<CreateJobRequest>,
    scheduler: web::Data<dyn JobScheduler>,
) -> Result<HttpResponse, ApiError> {
    let job_type = match body.job_type.as_str() {
        "calculate_depreciation" => JobType::CalculateDepreciation {
            asset_id: body.asset_id.unwrap_or(0),
            tenant_id: body.tenant_id,
        },
        "run_payroll" => JobType::RunPayroll {
            tenant_id: body.tenant_id,
            period: body.period.clone().unwrap_or_default(),
        },
        "send_reminders" => JobType::SendReminders {
            tenant_id: body.tenant_id,
        },
        "archive_logs" => JobType::ArchiveLogs {
            tenant_id: body.tenant_id,
            older_than_days: body.older_than_days.unwrap_or(30),
        },
        "generate_report" => JobType::GenerateReport {
            tenant_id: body.tenant_id,
            report_type: body.report_type.clone().unwrap_or_default(),
            params: body.params.clone().unwrap_or_default(),
        },
        "send_notification" => JobType::SendNotification {
            notification_id: body.asset_id.unwrap_or(0),
            tenant_id: body.tenant_id,
        },
        "custom" => JobType::Custom {
            name: body.custom_name.clone().unwrap_or_default(),
            payload: body.custom_payload.clone().unwrap_or_default(),
        },
        _ => return Err(ApiError::Validation("Unknown job type".to_string())),
    };

    let priority = match body.priority.as_deref() {
        Some("low") => JobPriority::Low,
        Some("high") => JobPriority::High,
        Some("critical") => JobPriority::Critical,
        _ => JobPriority::Normal,
    };

    let mut create = CreateJob::new(job_type, body.tenant_id)
        .with_priority(priority)
        .with_max_attempts(body.max_attempts.unwrap_or(3));

    if let Some(scheduled_at) = body.scheduled_at {
        create = create.with_scheduled_at(scheduled_at);
    }

    let job = scheduler
        .schedule(create)
        .await
        .map_err(ApiError::Internal)?;

    Ok(HttpResponse::Created().json(JobResponse::from_job(&job)))
}

/// Get next pending job (for worker processes)
#[utoipa::path(
    get,
    path = "/api/v1/jobs/next",
    tag = "Jobs",
    responses(
        (status = 200, description = "Next pending job", body = Option<JobResponse>),
        (status = 204, description = "No pending jobs"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn next_pending_job(
    _admin: AdminUser,
    scheduler: web::Data<dyn JobScheduler>,
) -> Result<HttpResponse, ApiError> {
    let job = scheduler.next_pending().await.map_err(ApiError::Internal)?;

    match job {
        Some(j) => Ok(HttpResponse::Ok().json(JobResponse::from_job(&j))),
        None => Ok(HttpResponse::NoContent().finish()),
    }
}

/// Get a job by ID
#[utoipa::path(
    get,
    path = "/api/v1/jobs/{id}",
    tag = "Jobs",
    params(("id" = i64, Path, description = "Job ID")),
    responses(
        (status = 200, description = "Job found", body = JobResponse),
        (status = 404, description = "Job not found"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_job(
    _admin: AdminUser,
    path: web::Path<i64>,
    scheduler: web::Data<dyn JobScheduler>,
) -> Result<HttpResponse, ApiError> {
    let id = path.into_inner();
    let job = scheduler.get_job(id).await.map_err(ApiError::Internal)?;

    match job {
        Some(j) => Ok(HttpResponse::Ok().json(JobResponse::from_job(&j))),
        None => Err(ApiError::NotFound("Job not found".to_string())),
    }
}

/// Mark a job as running
#[utoipa::path(
    post,
    path = "/api/v1/jobs/{id}/start",
    tag = "Jobs",
    params(("id" = i64, Path, description = "Job ID")),
    responses(
        (status = 200, description = "Job marked as running"),
        (status = 404, description = "Job not found"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn start_job(
    _admin: AdminUser,
    path: web::Path<i64>,
    scheduler: web::Data<dyn JobScheduler>,
) -> Result<HttpResponse, ApiError> {
    let id = path.into_inner();
    scheduler
        .mark_running(id)
        .await
        .map_err(ApiError::Internal)?;

    Ok(HttpResponse::Ok().json(serde_json::json!({"message": "Job marked as running"})))
}

/// Mark a job as completed
#[utoipa::path(
    post,
    path = "/api/v1/jobs/{id}/complete",
    tag = "Jobs",
    params(("id" = i64, Path, description = "Job ID")),
    responses(
        (status = 200, description = "Job marked as completed"),
        (status = 404, description = "Job not found"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn complete_job(
    _admin: AdminUser,
    path: web::Path<i64>,
    scheduler: web::Data<dyn JobScheduler>,
) -> Result<HttpResponse, ApiError> {
    let id = path.into_inner();
    scheduler
        .mark_completed(id)
        .await
        .map_err(ApiError::Internal)?;

    Ok(HttpResponse::Ok().json(serde_json::json!({"message": "Job completed"})))
}

/// Mark a job as failed
#[utoipa::path(
    post,
    path = "/api/v1/jobs/{id}/fail",
    tag = "Jobs",
    params(("id" = i64, Path, description = "Job ID")),
    request_body = FailJobRequest,
    responses(
        (status = 200, description = "Job marked as failed"),
        (status = 404, description = "Job not found"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn fail_job(
    _admin: AdminUser,
    path: web::Path<i64>,
    body: web::Json<FailJobRequest>,
    scheduler: web::Data<dyn JobScheduler>,
) -> Result<HttpResponse, ApiError> {
    let id = path.into_inner();
    scheduler
        .mark_failed(id, &body.error)
        .await
        .map_err(ApiError::Internal)?;

    Ok(HttpResponse::Ok().json(serde_json::json!({"message": "Job marked as failed"})))
}

/// Cancel a pending/scheduled job
#[utoipa::path(
    post,
    path = "/api/v1/jobs/{id}/cancel",
    tag = "Jobs",
    params(("id" = i64, Path, description = "Job ID")),
    responses(
        (status = 200, description = "Job cancelled"),
        (status = 404, description = "Job not found"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn cancel_job(
    _admin: AdminUser,
    path: web::Path<i64>,
    scheduler: web::Data<dyn JobScheduler>,
) -> Result<HttpResponse, ApiError> {
    let id = path.into_inner();
    scheduler.cancel(id).await.map_err(ApiError::Internal)?;

    Ok(HttpResponse::Ok().json(serde_json::json!({"message": "Job cancelled"})))
}

/// Retry a failed job
#[utoipa::path(
    post,
    path = "/api/v1/jobs/{id}/retry",
    tag = "Jobs",
    params(("id" = i64, Path, description = "Job ID")),
    responses(
        (status = 200, description = "Job retried"),
        (status = 404, description = "Job not found"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn retry_job(
    _admin: AdminUser,
    path: web::Path<i64>,
    scheduler: web::Data<dyn JobScheduler>,
) -> Result<HttpResponse, ApiError> {
    let id = path.into_inner();
    scheduler.retry(id).await.map_err(ApiError::Internal)?;

    Ok(HttpResponse::Ok().json(serde_json::json!({"message": "Job queued for retry"})))
}

/// List jobs by status
#[utoipa::path(
    get,
    path = "/api/v1/jobs/status/{status}",
    tag = "Jobs",
    params(("status" = String, Path, description = "Job status filter")),
    responses(
        (status = 200, description = "List of jobs", body = Vec<JobResponse>),
    ),
    security(("bearer_auth" = []))
)]
pub async fn list_jobs_by_status(
    _admin: AdminUser,
    path: web::Path<String>,
    _query: web::Query<PaginationParams>,
    scheduler: web::Data<dyn JobScheduler>,
) -> Result<HttpResponse, ApiError> {
    let status_str = path.into_inner();
    let status = match status_str.as_str() {
        "pending" => JobStatus::Pending,
        "running" => JobStatus::Running,
        "completed" => JobStatus::Completed,
        "failed" => JobStatus::Failed,
        "cancelled" => JobStatus::Cancelled,
        "scheduled" => JobStatus::Scheduled,
        _ => return Err(ApiError::Validation("Invalid job status".to_string())),
    };

    let jobs = scheduler
        .list_by_status(0, status)
        .await
        .map_err(ApiError::Internal)?;

    let responses: Vec<JobResponse> = jobs.iter().map(JobResponse::from_job).collect();
    Ok(HttpResponse::Ok().json(responses))
}

/// Cleanup old completed/failed jobs
#[utoipa::path(
    post,
    path = "/api/v1/jobs/cleanup/{days}",
    tag = "Jobs",
    params(("days" = u64, Path, description = "Remove jobs older than N days")),
    responses(
        (status = 200, description = "Jobs cleaned up"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn cleanup_jobs(
    _admin: AdminUser,
    path: web::Path<u64>,
    scheduler: web::Data<dyn JobScheduler>,
) -> Result<HttpResponse, ApiError> {
    let days = path.into_inner();
    let count = scheduler
        .cleanup(std::time::Duration::from_secs(days * 86400))
        .await
        .map_err(ApiError::Internal)?;

    Ok(HttpResponse::Ok().json(serde_json::json!({"cleaned": count})))
}

/// Configure jobs routes
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/v1/jobs")
            .route("", web::post().to(schedule_job))
            .route("/next", web::get().to(next_pending_job))
            .route("/status/{status}", web::get().to(list_jobs_by_status))
            .route("/cleanup/{days}", web::post().to(cleanup_jobs))
            .route("/{id}", web::get().to(get_job))
            .route("/{id}/start", web::post().to(start_job))
            .route("/{id}/complete", web::post().to(complete_job))
            .route("/{id}/fail", web::post().to(fail_job))
            .route("/{id}/cancel", web::post().to(cancel_job))
            .route("/{id}/retry", web::post().to(retry_job)),
    );
}
