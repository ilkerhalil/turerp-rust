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

        // Build dynamic WHERE clause
        let mut sql = String::from(
            "SELECT id, tenant_id, user_id, username, action, path, status_code, request_id, ip_address, user_agent, created_at, COUNT(*) OVER() as total_count FROM audit_logs WHERE tenant_id = $1",
        );
        let mut param_idx = 2u32;

        if query.user_id.is_some() {
            sql.push_str(&format!(" AND user_id = ${}", param_idx));
            param_idx += 1;
        }
        if query.path.is_some() {
            sql.push_str(&format!(" AND path ILIKE ${}", param_idx));
            param_idx += 1;
        }
        if query.from_date.is_some() {
            sql.push_str(&format!(" AND created_at >= ${}", param_idx));
            param_idx += 1;
        }
        if query.to_date.is_some() {
            sql.push_str(&format!(" AND created_at <= ${}", param_idx));
            param_idx += 1;
        }

        sql.push_str(&format!(
            " ORDER BY created_at DESC LIMIT ${} OFFSET ${}",
            param_idx,
            param_idx + 1
        ));

        let mut q = sqlx::query_as::<_, AuditLogRowWithTotal>(&sql).bind(tenant_id);

        if let Some(user_id) = query.user_id {
            q = q.bind(user_id);
        }
        if let Some(ref path) = query.path {
            q = q.bind(format!("%{}%", path));
        }
        if let Some(from_date) = query.from_date {
            q = q.bind(from_date);
        }
        if let Some(to_date) = query.to_date {
            q = q.bind(to_date);
        }

        q = q.bind(per_page as i64).bind(offset as i64);

        let rows = q
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

        // Build a batch INSERT with parameterized values
        let mut query_builder = String::from(
            "INSERT INTO audit_logs (tenant_id, user_id, username, action, path, status_code, request_id, ip_address, user_agent, created_at) VALUES ",
        );

        let chunks = logs.len();
        for i in 0..chunks {
            if i > 0 {
                query_builder.push_str(", ");
            }
            let base = i * 10;
            query_builder.push_str(&format!(
                "(${},{},{},{},{},{},{},{},{},{})",
                base + 1,
                base + 2,
                base + 3,
                base + 4,
                base + 5,
                base + 6,
                base + 7,
                base + 8,
                base + 9,
                base + 10
            ));
        }

        let mut q = sqlx::query(&query_builder);
        for log in logs {
            q = q
                .bind(log.tenant_id)
                .bind(log.user_id)
                .bind(log.username)
                .bind(log.action)
                .bind(log.path)
                .bind(log.status_code)
                .bind(log.request_id)
                .bind(log.ip_address)
                .bind(log.user_agent)
                .bind(log.created_at);
        }

        q.execute(&*self.pool)
            .await
            .map_err(|e| ApiError::Database(format!("Failed to batch insert audit logs: {}", e)))?;

        Ok(())
    }
}
