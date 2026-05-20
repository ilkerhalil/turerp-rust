//! PostgreSQL Vendor Portal repository implementations

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};
use std::sync::Arc;

use crate::db::error::map_sqlx_error;
use crate::domain::vendor_portal::model::{
    CreateDeliveryNote, CreateVendorUser, DeliveryNote, DeliveryNoteStatus, VendorUser,
    VendorUserStatus,
};
use crate::domain::vendor_portal::repository::{DeliveryNoteRepository, VendorUserRepository};
use crate::error::ApiError;

// ==================== VENDOR USER ====================

/// Database row representation for VendorUser
#[derive(Debug, FromRow)]
struct VendorUserRow {
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

impl From<VendorUserRow> for VendorUser {
    fn from(row: VendorUserRow) -> Self {
        let status = row.status.parse().unwrap_or_else(|e| {
            tracing::warn!(
                "Invalid vendor user status '{}' in database: {}, defaulting to Active",
                row.status,
                e
            );
            VendorUserStatus::Active
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

/// PostgreSQL vendor user repository
pub struct PostgresVendorUserRepository {
    pool: Arc<PgPool>,
}

impl PostgresVendorUserRepository {
    /// Create a new PostgreSQL vendor user repository
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    /// Convert to boxed trait object
    pub fn into_boxed(self) -> crate::domain::vendor_portal::repository::BoxVendorUserRepository {
        Arc::new(self) as crate::domain::vendor_portal::repository::BoxVendorUserRepository
    }
}

#[async_trait]
impl VendorUserRepository for PostgresVendorUserRepository {
    async fn create(
        &self,
        req: CreateVendorUser,
        password_hash: String,
        tenant_id: i64,
    ) -> Result<VendorUser, ApiError> {
        let language = req.language.unwrap_or_else(|| "en".to_string());
        let timezone = req
            .timezone
            .unwrap_or_else(|| "Europe/Istanbul".to_string());
        let status = VendorUserStatus::Active.to_string();

        let row: VendorUserRow = sqlx::query_as(
            r#"
            INSERT INTO vendor_users (tenant_id, cari_id, email, password_hash, full_name, phone, language, timezone, status, created_at, updated_at, last_login_at)
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
        .map_err(|e| map_sqlx_error(e, "Vendor user"))?;

        Ok(row.into())
    }

    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<VendorUser>, ApiError> {
        let result: Option<VendorUserRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, cari_id, email, password_hash, full_name, phone, language, timezone, status, created_at, updated_at, last_login_at
            FROM vendor_users
            WHERE id = $1 AND tenant_id = $2
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to get vendor user by id: {}", e)))?;

        Ok(result.map(|r| r.into()))
    }

    async fn find_by_email(
        &self,
        email: &str,
        tenant_id: i64,
    ) -> Result<Option<VendorUser>, ApiError> {
        let result: Option<VendorUserRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, cari_id, email, password_hash, full_name, phone, language, timezone, status, created_at, updated_at, last_login_at
            FROM vendor_users
            WHERE email = $1 AND tenant_id = $2
            "#,
        )
        .bind(email)
        .bind(tenant_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to get vendor user by email: {}", e)))?;

        Ok(result.map(|r| r.into()))
    }

    async fn find_by_cari(
        &self,
        cari_id: i64,
        tenant_id: i64,
    ) -> Result<Option<VendorUser>, ApiError> {
        let result: Option<VendorUserRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, cari_id, email, password_hash, full_name, phone, language, timezone, status, created_at, updated_at, last_login_at
            FROM vendor_users
            WHERE cari_id = $1 AND tenant_id = $2
            "#,
        )
        .bind(cari_id)
        .bind(tenant_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to get vendor user by cari: {}", e)))?;

        Ok(result.map(|r| r.into()))
    }

    async fn update_password(
        &self,
        id: i64,
        tenant_id: i64,
        password_hash: String,
    ) -> Result<VendorUser, ApiError> {
        let result: Option<VendorUserRow> = sqlx::query_as(
            r#"
            UPDATE vendor_users
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
        .map_err(|e| map_sqlx_error(e, "Vendor user"))?;

        result
            .map(|r| r.into())
            .ok_or_else(|| ApiError::NotFound(format!("Vendor user {} not found", id)))
    }

    async fn update_last_login(&self, id: i64, tenant_id: i64) -> Result<VendorUser, ApiError> {
        let result: Option<VendorUserRow> = sqlx::query_as(
            r#"
            UPDATE vendor_users
            SET last_login_at = NOW(), updated_at = NOW()
            WHERE id = $1 AND tenant_id = $2
            RETURNING id, tenant_id, cari_id, email, password_hash, full_name, phone, language, timezone, status, created_at, updated_at, last_login_at
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Vendor user"))?;

        result
            .map(|r| r.into())
            .ok_or_else(|| ApiError::NotFound(format!("Vendor user {} not found", id)))
    }

    async fn delete(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            DELETE FROM vendor_users
            WHERE id = $1 AND tenant_id = $2
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to delete vendor user: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound(format!("Vendor user {} not found", id)));
        }

        Ok(())
    }
}

// ==================== DELIVERY NOTE ====================

/// Database row representation for DeliveryNote
#[derive(Debug, FromRow)]
struct DeliveryNoteRow {
    id: i64,
    tenant_id: i64,
    vendor_user_id: i64,
    cari_id: i64,
    note_number: String,
    purchase_order_id: i64,
    description: String,
    status: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    shipped_at: Option<DateTime<Utc>>,
}

impl From<DeliveryNoteRow> for DeliveryNote {
    fn from(row: DeliveryNoteRow) -> Self {
        let status = row.status.parse().unwrap_or_else(|e| {
            tracing::warn!(
                "Invalid delivery note status '{}' in database: {}, defaulting to Draft",
                row.status,
                e
            );
            DeliveryNoteStatus::Draft
        });

        Self {
            id: row.id,
            tenant_id: row.tenant_id,
            vendor_user_id: row.vendor_user_id,
            cari_id: row.cari_id,
            note_number: row.note_number,
            purchase_order_id: row.purchase_order_id,
            description: row.description,
            status,
            created_at: row.created_at,
            updated_at: row.updated_at,
            shipped_at: row.shipped_at,
        }
    }
}

/// PostgreSQL delivery note repository
pub struct PostgresDeliveryNoteRepository {
    pool: Arc<PgPool>,
}

impl PostgresDeliveryNoteRepository {
    /// Create a new PostgreSQL delivery note repository
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    /// Convert to boxed trait object
    pub fn into_boxed(self) -> crate::domain::vendor_portal::repository::BoxDeliveryNoteRepository {
        Arc::new(self) as crate::domain::vendor_portal::repository::BoxDeliveryNoteRepository
    }
}

#[async_trait]
impl DeliveryNoteRepository for PostgresDeliveryNoteRepository {
    async fn create(
        &self,
        req: CreateDeliveryNote,
        vendor_user_id: i64,
        cari_id: i64,
        tenant_id: i64,
    ) -> Result<DeliveryNote, ApiError> {
        let note_number = format!("DN-{}", chrono::Utc::now().timestamp());
        let status = DeliveryNoteStatus::Draft.to_string();

        let row: DeliveryNoteRow = sqlx::query_as(
            r#"
            INSERT INTO delivery_notes (tenant_id, vendor_user_id, cari_id, note_number, purchase_order_id, description, status, created_at, updated_at, shipped_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, NOW(), NOW(), NULL)
            RETURNING id, tenant_id, vendor_user_id, cari_id, note_number, purchase_order_id, description, status, created_at, updated_at, shipped_at
            "#,
        )
        .bind(tenant_id)
        .bind(vendor_user_id)
        .bind(cari_id)
        .bind(&note_number)
        .bind(req.purchase_order_id)
        .bind(&req.description)
        .bind(&status)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Delivery note"))?;

        Ok(row.into())
    }

    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<DeliveryNote>, ApiError> {
        let result: Option<DeliveryNoteRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, vendor_user_id, cari_id, note_number, purchase_order_id, description, status, created_at, updated_at, shipped_at
            FROM delivery_notes
            WHERE id = $1 AND tenant_id = $2
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to get delivery note by id: {}", e)))?;

        Ok(result.map(|r| r.into()))
    }

    async fn find_by_cari(
        &self,
        cari_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<DeliveryNote>, ApiError> {
        let rows: Vec<DeliveryNoteRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, vendor_user_id, cari_id, note_number, purchase_order_id, description, status, created_at, updated_at, shipped_at
            FROM delivery_notes
            WHERE cari_id = $1 AND tenant_id = $2
            ORDER BY created_at DESC
            "#,
        )
        .bind(cari_id)
        .bind(tenant_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to get delivery notes by cari: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn find_by_vendor_user(
        &self,
        vendor_user_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<DeliveryNote>, ApiError> {
        let rows: Vec<DeliveryNoteRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, vendor_user_id, cari_id, note_number, purchase_order_id, description, status, created_at, updated_at, shipped_at
            FROM delivery_notes
            WHERE vendor_user_id = $1 AND tenant_id = $2
            ORDER BY created_at DESC
            "#,
        )
        .bind(vendor_user_id)
        .bind(tenant_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| {
            ApiError::Database(format!("Failed to get delivery notes by vendor user: {}", e))
        })?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<DeliveryNote>, ApiError> {
        let rows: Vec<DeliveryNoteRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, vendor_user_id, cari_id, note_number, purchase_order_id, description, status, created_at, updated_at, shipped_at
            FROM delivery_notes
            WHERE tenant_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to get delivery notes by tenant: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn update_status(
        &self,
        id: i64,
        tenant_id: i64,
        status: DeliveryNoteStatus,
    ) -> Result<DeliveryNote, ApiError> {
        let status_str = status.to_string();

        let result: Option<DeliveryNoteRow> = sqlx::query_as(
            r#"
            UPDATE delivery_notes
            SET status = $1,
                updated_at = NOW(),
                shipped_at = CASE
                    WHEN $1 = 'shipped' AND shipped_at IS NULL THEN NOW()
                    ELSE shipped_at
                END
            WHERE id = $2 AND tenant_id = $3
            RETURNING id, tenant_id, vendor_user_id, cari_id, note_number, purchase_order_id, description, status, created_at, updated_at, shipped_at
            "#,
        )
        .bind(&status_str)
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Delivery note"))?;

        result
            .map(|r| r.into())
            .ok_or_else(|| ApiError::NotFound(format!("Delivery note {} not found", id)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vendor_user_status_from_str() {
        assert_eq!(
            "active".parse::<VendorUserStatus>().unwrap(),
            VendorUserStatus::Active
        );
        assert_eq!(
            "passive".parse::<VendorUserStatus>().unwrap(),
            VendorUserStatus::Passive
        );
        assert_eq!(
            "blocked".parse::<VendorUserStatus>().unwrap(),
            VendorUserStatus::Blocked
        );
    }

    #[test]
    fn test_delivery_note_status_from_str() {
        assert_eq!(
            "draft".parse::<DeliveryNoteStatus>().unwrap(),
            DeliveryNoteStatus::Draft
        );
        assert_eq!(
            "shipped".parse::<DeliveryNoteStatus>().unwrap(),
            DeliveryNoteStatus::Shipped
        );
        assert_eq!(
            "partialreceived".parse::<DeliveryNoteStatus>().unwrap(),
            DeliveryNoteStatus::PartialReceived
        );
        assert_eq!(
            "received".parse::<DeliveryNoteStatus>().unwrap(),
            DeliveryNoteStatus::Received
        );
        assert_eq!(
            "cancelled".parse::<DeliveryNoteStatus>().unwrap(),
            DeliveryNoteStatus::Cancelled
        );
    }

    #[test]
    fn test_vendor_user_row_conversion_invalid_status() {
        let row = VendorUserRow {
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
        let user: VendorUser = row.into();
        assert_eq!(user.status, VendorUserStatus::Active);
    }

    #[test]
    fn test_delivery_note_row_conversion_invalid_status() {
        let row = DeliveryNoteRow {
            id: 1,
            tenant_id: 1,
            vendor_user_id: 1,
            cari_id: 1,
            note_number: "DN-1".to_string(),
            purchase_order_id: 1,
            description: "Desc".to_string(),
            status: "invalid".to_string(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            shipped_at: None,
        };
        let note: DeliveryNote = row.into();
        assert_eq!(note.status, DeliveryNoteStatus::Draft);
    }
}
