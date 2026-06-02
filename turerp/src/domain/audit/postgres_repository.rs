//! PostgreSQL audit log repository implementation

use async_trait::async_trait;
use sqlx::{FromRow, PgPool};
use std::sync::Arc;

use crate::common::pagination::PaginatedResult;
use crate::db::error::map_sqlx_error;
use crate::domain::audit::model::*;
use crate::domain::audit::repository::{AuditLogRepository, BoxAuditLogRepository};
use crate::error::ApiError;

/// Database row representation for AuditLog
#[derive(Debug, FromRow)]
struct AuditLogRow {
    id: i64,
    tenant_id: i64,
    user_id: i64,
    username: String,
    action: String,
    path: String,
    status_code: i16,
    request_id: String,
    ip_address: Option<String>,
    user_agent: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl From<AuditLogRow> for AuditLog {
    fn from(row: AuditLogRow) -> Self {
        Self {
            id: row.id,
            tenant_id: row.tenant_id,
            user_id: row.user_id,
            username: row.username,
            action: row.action,
            path: row.path,
            status_code: row.status_code,
            request_id: row.request_id,
            ip_address: row.ip_address,
            user_agent: row.user_agent,
            created_at: row.created_at,
        }
    }
}

/// Database row for paginated queries with total count
#[derive(Debug, FromRow)]
struct AuditLogRowWithTotal {
    id: i64,
    tenant_id: i64,
    user_id: i64,
    username: String,
    action: String,
    path: String,
    status_code: i16,
    request_id: String,
    ip_address: Option<String>,
    user_agent: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
    total_count: i64,
}

/// PostgreSQL audit log repository
pub struct PostgresAuditLogRepository {
    pool: Arc<PgPool>,
}

impl PostgresAuditLogRepository {
    /// Create a new PostgreSQL audit log repository
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    /// Convert to boxed trait object
    pub fn into_boxed(self) -> BoxAuditLogRepository {
        Arc::new(self) as BoxAuditLogRepository
    }
}

#[async_trait]
impl AuditLogRepository for PostgresAuditLogRepository {
    async fn create(&self, log: CreateAuditLog) -> Result<AuditLog, ApiError> {
        let row: AuditLogRow = sqlx::query_as(
            r#"
            INSERT INTO audit_logs (tenant_id, user_id, username, action, path, status_code, request_id, ip_address, user_agent, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING id, tenant_id, user_id, username, action, path, status_code, request_id, ip_address, user_agent, created_at
            "#,
        )
        .bind(log.tenant_id)
        .bind(log.user_id)
        .bind(&log.username)
        .bind(&log.action)
        .bind(&log.path)
        .bind(log.status_code)
        .bind(&log.request_id)
        .bind(&log.ip_address)
        .bind(&log.user_agent)
        .bind(log.created_at)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "AuditLog"))?;

        Ok(row.into())
    }

    async fn find_by_tenant_paginated(
        &self,
        tenant_id: i64,
        query: AuditLogQueryParams,
    ) -> Result<PaginatedResult<AuditLog>, ApiError> {
        let page = query.page.max(1);
        let per_page = query.per_page.max(1);
        let offset = (page.saturating_sub(1)) * per_page;

        // Build dynamic WHERE clause with QueryBuilder so all filter values are
        // bound parameters (no string interpolation of user input).
        let mut qb: sqlx::QueryBuilder<sqlx::Postgres> = sqlx::QueryBuilder::new(
            "SELECT id, tenant_id, user_id, username, action, path, status_code, request_id, \
             ip_address, user_agent, created_at, COUNT(*) OVER() AS total_count \
             FROM audit_logs WHERE tenant_id = ",
        );
        qb.push_bind(tenant_id);

        if let Some(user_id) = query.user_id {
            qb.push(" AND user_id = ").push_bind(user_id);
        }
        if let Some(ref path) = query.path {
            qb.push(" AND path ILIKE ").push_bind(format!("%{}%", path));
        }
        if let Some(from_date) = query.from_date {
            qb.push(" AND created_at >= ").push_bind(from_date);
        }
        if let Some(to_date) = query.to_date {
            qb.push(" AND created_at <= ").push_bind(to_date);
        }

        qb.push(" ORDER BY created_at DESC LIMIT ")
            .push_bind(per_page as i64)
            .push(" OFFSET ")
            .push_bind(offset as i64);

        let rows: Vec<AuditLogRowWithTotal> = qb
            .build_query_as()
            .fetch_all(&*self.pool)
            .await
            .map_err(|e| map_sqlx_error(e, "AuditLog"))?;

        let total = rows.first().map(|r| r.total_count as u64).unwrap_or(0);
        let items: Vec<AuditLog> = rows
            .into_iter()
            .map(|r| AuditLog {
                id: r.id,
                tenant_id: r.tenant_id,
                user_id: r.user_id,
                username: r.username,
                action: r.action,
                path: r.path,
                status_code: r.status_code,
                request_id: r.request_id,
                ip_address: r.ip_address,
                user_agent: r.user_agent,
                created_at: r.created_at,
            })
            .collect();

        Ok(PaginatedResult::new(items, page, per_page, total))
    }

    async fn create_batch(&self, logs: Vec<CreateAuditLog>) -> Result<(), ApiError> {
        if logs.is_empty() {
            return Ok(());
        }

        // Build a batch INSERT using QueryBuilder::push_values so the row
        // placeholders and per-row parameter positions are generated safely.
        let mut qb: sqlx::QueryBuilder<sqlx::Postgres> = sqlx::QueryBuilder::new(
            "INSERT INTO audit_logs \
             (tenant_id, user_id, username, action, path, status_code, request_id, ip_address, user_agent, created_at) ",
        );

        qb.push_values(logs, |mut b, log| {
            b.push_bind(log.tenant_id)
                .push_bind(log.user_id)
                .push_bind(log.username)
                .push_bind(log.action)
                .push_bind(log.path)
                .push_bind(log.status_code)
                .push_bind(log.request_id)
                .push_bind(log.ip_address)
                .push_bind(log.user_agent)
                .push_bind(log.created_at);
        });

        qb.build()
            .execute(&*self.pool)
            .await
            .map_err(|e| ApiError::Database(format!("Failed to batch insert audit logs: {}", e)))?;

        Ok(())
    }
}
