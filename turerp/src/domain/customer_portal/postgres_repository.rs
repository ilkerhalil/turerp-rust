//! PostgreSQL Customer Portal repository implementations

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};
use std::sync::Arc;

use crate::db::error::map_sqlx_error;
use crate::domain::customer_portal::model::{
    CreatePortalUser, CreateSupportTicket, PortalUser, PortalUserStatus, SupportTicket,
    SupportTicketStatus, TicketCategory, TicketPriority,
};
use crate::domain::customer_portal::repository::{PortalUserRepository, SupportTicketRepository};
use crate::error::ApiError;

// ==================== PORTAL USER ====================

/// Database row representation for PortalUser
#[derive(Debug, FromRow)]
struct PortalUserRow {
    id: i64,
    tenant_id: i64,
    cari_id: i64,
    email: String,
    password_hash: String,
    full_name: String,
    phone: Option<String>,
    language: String,
    timezone: String,
    status: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    last_login_at: Option<DateTime<Utc>>,
}

impl From<PortalUserRow> for PortalUser {
    fn from(row: PortalUserRow) -> Self {
        let status = row.status.parse().unwrap_or_else(|e| {
            tracing::warn!(
                "Invalid portal user status '{}' in database: {}, defaulting to Active",
                row.status,
                e
            );
            PortalUserStatus::Active
        });

        Self {
            id: row.id,
            tenant_id: row.tenant_id,
            cari_id: row.cari_id,
            email: row.email,
            password_hash: row.password_hash,
            full_name: row.full_name,
            phone: row.phone,
            language: row.language,
            timezone: row.timezone,
            status,
            created_at: row.created_at,
            updated_at: row.updated_at,
            last_login_at: row.last_login_at,
        }
    }
}

/// PostgreSQL portal user repository
pub struct PostgresPortalUserRepository {
    pool: Arc<PgPool>,
}

impl PostgresPortalUserRepository {
    /// Create a new PostgreSQL portal user repository
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    /// Convert to boxed trait object
    pub fn into_boxed(self) -> crate::domain::customer_portal::repository::BoxPortalUserRepository {
        Arc::new(self) as crate::domain::customer_portal::repository::BoxPortalUserRepository
    }
}

#[async_trait]
impl PortalUserRepository for PostgresPortalUserRepository {
    async fn create(
        &self,
        req: CreatePortalUser,
        password_hash: String,
        tenant_id: i64,
    ) -> Result<PortalUser, ApiError> {
        let language = req.language.unwrap_or_else(|| "en".to_string());
        let timezone = req
            .timezone
            .unwrap_or_else(|| "Europe/Istanbul".to_string());
        let status = PortalUserStatus::Active.to_string();

        let row: PortalUserRow = sqlx::query_as(
            r#"
            INSERT INTO portal_users (tenant_id, cari_id, email, password_hash, full_name, phone, language, timezone, status, created_at, updated_at, last_login_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, NOW(), NOW(), NULL)
            RETURNING id, tenant_id, cari_id, email, password_hash, full_name, phone, language, timezone, status, created_at, updated_at, last_login_at
            "#,
        )
        .bind(tenant_id)
        .bind(req.cari_id)
        .bind(&req.email)
        .bind(&password_hash)
        .bind(&req.full_name)
        .bind(&req.phone)
        .bind(&language)
        .bind(&timezone)
        .bind(&status)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Portal user"))?;

        Ok(row.into())
    }

    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<PortalUser>, ApiError> {
        let result: Option<PortalUserRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, cari_id, email, password_hash, full_name, phone, language, timezone, status, created_at, updated_at, last_login_at
            FROM portal_users
            WHERE id = $1 AND tenant_id = $2
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to get portal user by id: {}", e)))?;

        Ok(result.map(|r| r.into()))
    }

    async fn find_by_email(
        &self,
        email: &str,
        tenant_id: i64,
    ) -> Result<Option<PortalUser>, ApiError> {
        let result: Option<PortalUserRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, cari_id, email, password_hash, full_name, phone, language, timezone, status, created_at, updated_at, last_login_at
            FROM portal_users
            WHERE email = $1 AND tenant_id = $2
            "#,
        )
        .bind(email)
        .bind(tenant_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to get portal user by email: {}", e)))?;

        Ok(result.map(|r| r.into()))
    }

    async fn find_by_cari(
        &self,
        cari_id: i64,
        tenant_id: i64,
    ) -> Result<Option<PortalUser>, ApiError> {
        let result: Option<PortalUserRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, cari_id, email, password_hash, full_name, phone, language, timezone, status, created_at, updated_at, last_login_at
            FROM portal_users
            WHERE cari_id = $1 AND tenant_id = $2
            "#,
        )
        .bind(cari_id)
        .bind(tenant_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to get portal user by cari: {}", e)))?;

        Ok(result.map(|r| r.into()))
    }

    async fn update_password(
        &self,
        id: i64,
        tenant_id: i64,
        password_hash: String,
    ) -> Result<PortalUser, ApiError> {
        let result: Option<PortalUserRow> = sqlx::query_as(
            r#"
            UPDATE portal_users
            SET password_hash = $1, updated_at = NOW()
            WHERE id = $2 AND tenant_id = $3
            RETURNING id, tenant_id, cari_id, email, password_hash, full_name, phone, language, timezone, status, created_at, updated_at, last_login_at
            "#,
        )
        .bind(&password_hash)
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Portal user"))?;

        result
            .map(|r| r.into())
            .ok_or_else(|| ApiError::NotFound(format!("Portal user {} not found", id)))
    }

    async fn update_last_login(&self, id: i64, tenant_id: i64) -> Result<PortalUser, ApiError> {
        let result: Option<PortalUserRow> = sqlx::query_as(
            r#"
            UPDATE portal_users
            SET last_login_at = NOW(), updated_at = NOW()
            WHERE id = $1 AND tenant_id = $2
            RETURNING id, tenant_id, cari_id, email, password_hash, full_name, phone, language, timezone, status, created_at, updated_at, last_login_at
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Portal user"))?;

        result
            .map(|r| r.into())
            .ok_or_else(|| ApiError::NotFound(format!("Portal user {} not found", id)))
    }

    async fn delete(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            DELETE FROM portal_users
            WHERE id = $1 AND tenant_id = $2
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to delete portal user: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound(format!("Portal user {} not found", id)));
        }

        Ok(())
    }
}

// ==================== SUPPORT TICKET ====================

/// Database row representation for SupportTicket
#[derive(Debug, FromRow)]
struct SupportTicketRow {
    id: i64,
    tenant_id: i64,
    portal_user_id: i64,
    cari_id: i64,
    ticket_number: String,
    subject: String,
    description: String,
    status: String,
    priority: String,
    category: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    resolved_at: Option<DateTime<Utc>>,
}

impl From<SupportTicketRow> for SupportTicket {
    fn from(row: SupportTicketRow) -> Self {
        let status = row.status.parse().unwrap_or_else(|e| {
            tracing::warn!(
                "Invalid support ticket status '{}' in database: {}, defaulting to Open",
                row.status,
                e
            );
            SupportTicketStatus::Open
        });
        let priority = row.priority.parse().unwrap_or_else(|e| {
            tracing::warn!(
                "Invalid ticket priority '{}' in database: {}, defaulting to Medium",
                row.priority,
                e
            );
            TicketPriority::Medium
        });
        let category = row.category.parse().unwrap_or_else(|e| {
            tracing::warn!(
                "Invalid ticket category '{}' in database: {}, defaulting to General",
                row.category,
                e
            );
            TicketCategory::General
        });

        Self {
            id: row.id,
            tenant_id: row.tenant_id,
            portal_user_id: row.portal_user_id,
            cari_id: row.cari_id,
            ticket_number: row.ticket_number,
            subject: row.subject,
            description: row.description,
            status,
            priority,
            category,
            created_at: row.created_at,
            updated_at: row.updated_at,
            resolved_at: row.resolved_at,
        }
    }
}

/// PostgreSQL support ticket repository
pub struct PostgresSupportTicketRepository {
    pool: Arc<PgPool>,
}

impl PostgresSupportTicketRepository {
    /// Create a new PostgreSQL support ticket repository
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    /// Convert to boxed trait object
    pub fn into_boxed(
        self,
    ) -> crate::domain::customer_portal::repository::BoxSupportTicketRepository {
        Arc::new(self) as crate::domain::customer_portal::repository::BoxSupportTicketRepository
    }
}

#[async_trait]
impl SupportTicketRepository for PostgresSupportTicketRepository {
    async fn create(
        &self,
        req: CreateSupportTicket,
        portal_user_id: i64,
        cari_id: i64,
        tenant_id: i64,
    ) -> Result<SupportTicket, ApiError> {
        let ticket_number = format!("TKT-{}", chrono::Utc::now().timestamp());
        let status = SupportTicketStatus::Open.to_string();
        let priority = req.priority.to_string();
        let category = req.category.to_string();

        let row: SupportTicketRow = sqlx::query_as(
            r#"
            INSERT INTO support_tickets (tenant_id, portal_user_id, cari_id, ticket_number, subject, description, status, priority, category, created_at, updated_at, resolved_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, NOW(), NOW(), NULL)
            RETURNING id, tenant_id, portal_user_id, cari_id, ticket_number, subject, description, status, priority, category, created_at, updated_at, resolved_at
            "#,
        )
        .bind(tenant_id)
        .bind(portal_user_id)
        .bind(cari_id)
        .bind(&ticket_number)
        .bind(&req.subject)
        .bind(&req.description)
        .bind(&status)
        .bind(&priority)
        .bind(&category)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Support ticket"))?;

        Ok(row.into())
    }

    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<SupportTicket>, ApiError> {
        let result: Option<SupportTicketRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, portal_user_id, cari_id, ticket_number, subject, description, status, priority, category, created_at, updated_at, resolved_at
            FROM support_tickets
            WHERE id = $1 AND tenant_id = $2
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to get support ticket by id: {}", e)))?;

        Ok(result.map(|r| r.into()))
    }

    async fn find_by_cari(
        &self,
        cari_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<SupportTicket>, ApiError> {
        let rows: Vec<SupportTicketRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, portal_user_id, cari_id, ticket_number, subject, description, status, priority, category, created_at, updated_at, resolved_at
            FROM support_tickets
            WHERE cari_id = $1 AND tenant_id = $2
            ORDER BY created_at DESC
            "#,
        )
        .bind(cari_id)
        .bind(tenant_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to get support tickets by cari: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn find_by_portal_user(
        &self,
        portal_user_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<SupportTicket>, ApiError> {
        let rows: Vec<SupportTicketRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, portal_user_id, cari_id, ticket_number, subject, description, status, priority, category, created_at, updated_at, resolved_at
            FROM support_tickets
            WHERE portal_user_id = $1 AND tenant_id = $2
            ORDER BY created_at DESC
            "#,
        )
        .bind(portal_user_id)
        .bind(tenant_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to get support tickets by portal user: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<SupportTicket>, ApiError> {
        let rows: Vec<SupportTicketRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, portal_user_id, cari_id, ticket_number, subject, description, status, priority, category, created_at, updated_at, resolved_at
            FROM support_tickets
            WHERE tenant_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to get support tickets by tenant: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn update_status(
        &self,
        id: i64,
        tenant_id: i64,
        status: SupportTicketStatus,
    ) -> Result<SupportTicket, ApiError> {
        let status_str = status.to_string();

        let result: Option<SupportTicketRow> = sqlx::query_as(
            r#"
            UPDATE support_tickets
            SET status = $1, updated_at = NOW()
            WHERE id = $2 AND tenant_id = $3
            RETURNING id, tenant_id, portal_user_id, cari_id, ticket_number, subject, description, status, priority, category, created_at, updated_at, resolved_at
            "#,
        )
        .bind(&status_str)
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Support ticket"))?;

        result
            .map(|r| r.into())
            .ok_or_else(|| ApiError::NotFound(format!("Support ticket {} not found", id)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_portal_user_status_from_str() {
        assert_eq!(
            "active".parse::<PortalUserStatus>().unwrap(),
            PortalUserStatus::Active
        );
        assert_eq!(
            "passive".parse::<PortalUserStatus>().unwrap(),
            PortalUserStatus::Passive
        );
        assert_eq!(
            "blocked".parse::<PortalUserStatus>().unwrap(),
            PortalUserStatus::Blocked
        );
    }

    #[test]
    fn test_support_ticket_status_from_str() {
        assert_eq!(
            "open".parse::<SupportTicketStatus>().unwrap(),
            SupportTicketStatus::Open
        );
        assert_eq!(
            "inprogress".parse::<SupportTicketStatus>().unwrap(),
            SupportTicketStatus::InProgress
        );
        assert_eq!(
            "resolved".parse::<SupportTicketStatus>().unwrap(),
            SupportTicketStatus::Resolved
        );
    }

    #[test]
    fn test_ticket_priority_from_str() {
        assert_eq!(
            "low".parse::<TicketPriority>().unwrap(),
            TicketPriority::Low
        );
        assert_eq!(
            "medium".parse::<TicketPriority>().unwrap(),
            TicketPriority::Medium
        );
        assert_eq!(
            "high".parse::<TicketPriority>().unwrap(),
            TicketPriority::High
        );
        assert_eq!(
            "critical".parse::<TicketPriority>().unwrap(),
            TicketPriority::Critical
        );
    }

    #[test]
    fn test_ticket_category_from_str() {
        assert_eq!(
            "general".parse::<TicketCategory>().unwrap(),
            TicketCategory::General
        );
        assert_eq!(
            "order".parse::<TicketCategory>().unwrap(),
            TicketCategory::Order
        );
        assert_eq!(
            "technical".parse::<TicketCategory>().unwrap(),
            TicketCategory::Technical
        );
    }

    #[test]
    fn test_portal_user_row_conversion_invalid_status() {
        let row = PortalUserRow {
            id: 1,
            tenant_id: 1,
            cari_id: 1,
            email: "test@example.com".to_string(),
            password_hash: "hash".to_string(),
            full_name: "Test User".to_string(),
            phone: None,
            language: "en".to_string(),
            timezone: "UTC".to_string(),
            status: "invalid_status".to_string(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            last_login_at: None,
        };
        let user: PortalUser = row.into();
        assert_eq!(user.status, PortalUserStatus::Active);
    }

    #[test]
    fn test_support_ticket_row_conversion_invalid_fields() {
        let row = SupportTicketRow {
            id: 1,
            tenant_id: 1,
            portal_user_id: 1,
            cari_id: 1,
            ticket_number: "TKT-1".to_string(),
            subject: "Test".to_string(),
            description: "Description".to_string(),
            status: "invalid".to_string(),
            priority: "invalid".to_string(),
            category: "invalid".to_string(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            resolved_at: None,
        };
        let ticket: SupportTicket = row.into();
        assert_eq!(ticket.status, SupportTicketStatus::Open);
        assert_eq!(ticket.priority, TicketPriority::Medium);
        assert_eq!(ticket.category, TicketCategory::General);
    }
}
