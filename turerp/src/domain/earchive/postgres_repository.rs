//! PostgreSQL E-Archive repository implementation

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};
use std::sync::Arc;

use crate::common::pagination::{PaginatedResult, PaginationParams};
use crate::db::error::map_sqlx_error;
use crate::domain::earchive::model::{EarchiveDocument, EarchiveStatus, EarchiveType};
use crate::domain::earchive::repository::{BoxEarchiveRepository, EarchiveRepository};
use crate::error::ApiError;

// ---------------------------------------------------------------------------
// EarchiveDocumentRow
// ---------------------------------------------------------------------------

/// Database row representation for EarchiveDocument
#[derive(Debug, FromRow)]
struct EarchiveDocumentRow {
    id: i64,
    tenant_id: i64,
    document_type: String,
    related_invoice_id: Option<i64>,
    uuid: String,
    xml_content: String,
    signature: Option<String>,
    status: String,
    gib_response: Option<String>,
    error_message: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    sent_at: Option<DateTime<Utc>>,
    total_count: Option<i64>,
}

impl From<EarchiveDocumentRow> for EarchiveDocument {
    fn from(row: EarchiveDocumentRow) -> Self {
        Self {
            id: row.id,
            tenant_id: row.tenant_id,
            document_type: row.document_type.parse().unwrap_or_else(|e| {
                tracing::warn!(error = %e, "Invalid document_type in database");
                EarchiveType::EArchiveInvoice
            }),
            related_invoice_id: row.related_invoice_id,
            uuid: row.uuid,
            xml_content: row.xml_content,
            signature: row.signature,
            status: row.status.parse().unwrap_or_else(|e| {
                tracing::warn!(error = %e, "Invalid status in database");
                EarchiveStatus::Draft
            }),
            gib_response: row.gib_response,
            error_message: row.error_message,
            created_at: row.created_at,
            updated_at: row.updated_at,
            sent_at: row.sent_at,
        }
    }
}

// ---------------------------------------------------------------------------
// PostgresEarchiveRepository
// ---------------------------------------------------------------------------

/// PostgreSQL E-Archive repository
pub struct PostgresEarchiveRepository {
    pool: Arc<PgPool>,
}

impl PostgresEarchiveRepository {
    /// Create a new PostgreSQL E-Archive repository
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    /// Convert to boxed trait object
    pub fn into_boxed(self) -> BoxEarchiveRepository {
        Arc::new(self) as BoxEarchiveRepository
    }
}

#[async_trait]
impl EarchiveRepository for PostgresEarchiveRepository {
    async fn create(&self, doc: EarchiveDocument) -> Result<EarchiveDocument, ApiError> {
        let row: EarchiveDocumentRow = sqlx::query_as(
            r#"
            INSERT INTO earchive_documents (
                tenant_id, document_type, related_invoice_id, uuid,
                xml_content, signature, status, gib_response, error_message,
                sent_at, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, NOW(), NOW())
            RETURNING
                id, tenant_id, document_type, related_invoice_id, uuid,
                xml_content, signature, status, gib_response, error_message,
                created_at, updated_at, sent_at
            "#,
        )
        .bind(doc.tenant_id)
        .bind(doc.document_type.to_string())
        .bind(doc.related_invoice_id)
        .bind(&doc.uuid)
        .bind(&doc.xml_content)
        .bind(&doc.signature)
        .bind(doc.status.to_string())
        .bind(&doc.gib_response)
        .bind(&doc.error_message)
        .bind(doc.sent_at)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "E-Archive document"))?;

        Ok(row.into())
    }

    async fn find_by_id(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<Option<EarchiveDocument>, ApiError> {
        let result: Option<EarchiveDocumentRow> = sqlx::query_as(
            r#"
            SELECT
                id, tenant_id, document_type, related_invoice_id, uuid,
                xml_content, signature, status, gib_response, error_message,
                created_at, updated_at, sent_at
            FROM earchive_documents
            WHERE id = $1 AND tenant_id = $2
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| {
            ApiError::Database(format!("Failed to get E-Archive document by id: {}", e))
        })?;

        Ok(result.map(|r| r.into()))
    }

    async fn find_by_uuid(
        &self,
        uuid: &str,
        tenant_id: i64,
    ) -> Result<Option<EarchiveDocument>, ApiError> {
        let result: Option<EarchiveDocumentRow> = sqlx::query_as(
            r#"
            SELECT
                id, tenant_id, document_type, related_invoice_id, uuid,
                xml_content, signature, status, gib_response, error_message,
                created_at, updated_at, sent_at
            FROM earchive_documents
            WHERE uuid = $1 AND tenant_id = $2
            "#,
        )
        .bind(uuid)
        .bind(tenant_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| {
            ApiError::Database(format!("Failed to get E-Archive document by uuid: {}", e))
        })?;

        Ok(result.map(|r| r.into()))
    }

    async fn find_by_tenant(
        &self,
        tenant_id: i64,
        status: Option<EarchiveStatus>,
        params: PaginationParams,
    ) -> Result<PaginatedResult<EarchiveDocument>, ApiError> {
        let offset = (params.page.saturating_sub(1)) * params.per_page;
        let status_filter = status.map(|s| s.to_string());

        let rows: Vec<EarchiveDocumentRow> = sqlx::query_as(
            r#"
            SELECT
                id, tenant_id, document_type, related_invoice_id, uuid,
                xml_content, signature, status, gib_response, error_message,
                created_at, updated_at, sent_at,
                COUNT(*) OVER() as total_count
            FROM earchive_documents
            WHERE tenant_id = $1
              AND ($2::varchar IS NULL OR status = $2)
            ORDER BY id DESC
            LIMIT $3 OFFSET $4
            "#,
        )
        .bind(tenant_id)
        .bind(&status_filter)
        .bind(params.per_page as i64)
        .bind(offset as i64)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| {
            ApiError::Database(format!(
                "Failed to get E-Archive documents by tenant: {}",
                e
            ))
        })?;

        let total = rows.first().and_then(|r| r.total_count).unwrap_or(0) as u64;
        let items: Vec<EarchiveDocument> = rows.into_iter().map(|r| r.into()).collect();
        Ok(PaginatedResult::new(
            items,
            params.page,
            params.per_page,
            total,
        ))
    }

    async fn update_status(
        &self,
        id: i64,
        tenant_id: i64,
        status: EarchiveStatus,
        gib_response: Option<String>,
        error_message: Option<String>,
        sent_at: Option<DateTime<Utc>>,
    ) -> Result<EarchiveDocument, ApiError> {
        let result: Option<EarchiveDocumentRow> = sqlx::query_as(
            r#"
            UPDATE earchive_documents
            SET
                status = $3,
                gib_response = COALESCE($4, gib_response),
                error_message = COALESCE($5, error_message),
                sent_at = COALESCE($6, sent_at),
                updated_at = NOW()
            WHERE id = $1 AND tenant_id = $2
            RETURNING
                id, tenant_id, document_type, related_invoice_id, uuid,
                xml_content, signature, status, gib_response, error_message,
                created_at, updated_at, sent_at
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .bind(status.to_string())
        .bind(&gib_response)
        .bind(&error_message)
        .bind(sent_at)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "E-Archive document"))?;

        result
            .map(|r| r.into())
            .ok_or_else(|| ApiError::NotFound(format!("E-Archive document {} not found", id)))
    }

    async fn list_by_status(
        &self,
        tenant_id: i64,
        status: EarchiveStatus,
        params: PaginationParams,
    ) -> Result<PaginatedResult<EarchiveDocument>, ApiError> {
        self.find_by_tenant(tenant_id, Some(status), params).await
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn sample_row() -> EarchiveDocumentRow {
        EarchiveDocumentRow {
            id: 1,
            tenant_id: 100,
            document_type: "EArchiveInvoice".to_string(),
            related_invoice_id: Some(42),
            uuid: "uuid-123".to_string(),
            xml_content: "<xml/>".to_string(),
            signature: Some("sig".to_string()),
            status: "Draft".to_string(),
            gib_response: Some("200".to_string()),
            error_message: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            sent_at: None,
            total_count: Some(10),
        }
    }

    #[test]
    fn test_row_conversion() {
        let row = sample_row();
        let doc: EarchiveDocument = row.into();

        assert_eq!(doc.id, 1);
        assert_eq!(doc.tenant_id, 100);
        assert_eq!(doc.document_type, EarchiveType::EArchiveInvoice);
        assert_eq!(doc.related_invoice_id, Some(42));
        assert_eq!(doc.uuid, "uuid-123");
        assert_eq!(doc.xml_content, "<xml/>");
        assert_eq!(doc.signature, Some("sig".to_string()));
        assert_eq!(doc.status, EarchiveStatus::Draft);
        assert_eq!(doc.gib_response, Some("200".to_string()));
        assert!(doc.error_message.is_none());
        assert!(doc.sent_at.is_none());
    }

    #[test]
    fn test_row_conversion_eserbest_makbuzu() {
        let mut row = sample_row();
        row.document_type = "ESerbestMeslekMakbuzu".to_string();
        row.status = "Sent".to_string();

        let doc: EarchiveDocument = row.into();
        assert_eq!(doc.document_type, EarchiveType::ESerbestMeslekMakbuzu);
        assert_eq!(doc.status, EarchiveStatus::Sent);
    }

    #[test]
    fn test_row_conversion_invalid_type_fallback() {
        let mut row = sample_row();
        row.document_type = "InvalidType".to_string();

        let doc: EarchiveDocument = row.into();
        assert_eq!(doc.document_type, EarchiveType::EArchiveInvoice);
    }

    #[test]
    fn test_row_conversion_invalid_status_fallback() {
        let mut row = sample_row();
        row.status = "InvalidStatus".to_string();

        let doc: EarchiveDocument = row.into();
        assert_eq!(doc.status, EarchiveStatus::Draft);
    }

    #[test]
    fn test_all_status_variants() {
        for status in [
            EarchiveStatus::Draft,
            EarchiveStatus::Generated,
            EarchiveStatus::Signed,
            EarchiveStatus::Sent,
            EarchiveStatus::Accepted,
            EarchiveStatus::Rejected,
            EarchiveStatus::Cancelled,
        ] {
            let mut row = sample_row();
            row.status = status.to_string();
            let doc: EarchiveDocument = row.into();
            assert_eq!(doc.status, status);
        }
    }
}
