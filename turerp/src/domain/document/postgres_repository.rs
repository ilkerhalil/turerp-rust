//! PostgreSQL document repository implementation

use async_trait::async_trait;
use sqlx::{FromRow, PgPool};
use std::sync::Arc;

use crate::db::error::map_sqlx_error;
use crate::domain::document::model::{
    CreateDocument, CreateDocumentCategory, CreateDocumentLink, CreateDocumentVersion, Document,
    DocumentCategory, DocumentLink, DocumentSearchParams, DocumentSearchResult, DocumentVersion,
    UpdateDocument, UpdateDocumentCategory,
};
use crate::domain::document::repository::{BoxDocumentRepository, DocumentRepository};
use crate::error::ApiError;

/// Database row representation for Document
#[derive(Debug, FromRow)]
struct DocumentRow {
    id: i64,
    tenant_id: i64,
    name: String,
    filename: String,
    size_bytes: i64,
    mime_type: String,
    hash: String,
    storage_path: String,
    uploaded_by: Option<i64>,
    category_id: Option<i64>,
    tags: Option<Vec<String>>,
    description: Option<String>,
    current_version: i32,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
    deleted_at: Option<chrono::DateTime<chrono::Utc>>,
    deleted_by: Option<i64>,
}

impl From<DocumentRow> for Document {
    fn from(row: DocumentRow) -> Self {
        Self {
            id: row.id,
            tenant_id: row.tenant_id,
            name: row.name,
            filename: row.filename,
            size_bytes: row.size_bytes,
            mime_type: row.mime_type,
            hash: row.hash,
            storage_path: row.storage_path,
            uploaded_by: row.uploaded_by,
            category_id: row.category_id,
            tags: row.tags.unwrap_or_default(),
            description: row.description,
            current_version: row.current_version,
            created_at: row.created_at,
            updated_at: row.updated_at,
            deleted_at: row.deleted_at,
            deleted_by: row.deleted_by,
        }
    }
}

/// Database row representation for DocumentVersion
#[derive(Debug, FromRow)]
struct DocumentVersionRow {
    id: i64,
    document_id: i64,
    tenant_id: i64,
    version_number: i32,
    filename: String,
    size_bytes: i64,
    hash: String,
    storage_path: String,
    created_by: Option<i64>,
    comment: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl From<DocumentVersionRow> for DocumentVersion {
    fn from(row: DocumentVersionRow) -> Self {
        Self {
            id: row.id,
            document_id: row.document_id,
            tenant_id: row.tenant_id,
            version_number: row.version_number,
            filename: row.filename,
            size_bytes: row.size_bytes,
            hash: row.hash,
            storage_path: row.storage_path,
            created_by: row.created_by,
            comment: row.comment,
            created_at: row.created_at,
        }
    }
}

/// Database row representation for DocumentCategory
#[derive(Debug, FromRow)]
struct DocumentCategoryRow {
    id: i64,
    tenant_id: i64,
    name: String,
    description: Option<String>,
    color: Option<String>,
    parent_id: Option<i64>,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<DocumentCategoryRow> for DocumentCategory {
    fn from(row: DocumentCategoryRow) -> Self {
        Self {
            id: row.id,
            tenant_id: row.tenant_id,
            name: row.name,
            description: row.description,
            color: row.color,
            parent_id: row.parent_id,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

/// Database row representation for DocumentLink
#[derive(Debug, FromRow)]
struct DocumentLinkRow {
    id: i64,
    document_id: i64,
    tenant_id: i64,
    entity_type: String,
    entity_id: i64,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl From<DocumentLinkRow> for DocumentLink {
    fn from(row: DocumentLinkRow) -> Self {
        Self {
            id: row.id,
            document_id: row.document_id,
            tenant_id: row.tenant_id,
            entity_type: row.entity_type,
            entity_id: row.entity_id,
            created_at: row.created_at,
        }
    }
}

/// PostgreSQL implementation of DocumentRepository
pub struct PostgresDocumentRepository {
    pool: Arc<PgPool>,
}

impl PostgresDocumentRepository {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    pub fn into_boxed(self) -> BoxDocumentRepository {
        Arc::new(self)
    }
}

#[async_trait]
impl DocumentRepository for PostgresDocumentRepository {
    // --- Document CRUD ---

    async fn create(&self, doc: CreateDocument) -> Result<Document, ApiError> {
        let row = sqlx::query_as::<_, DocumentRow>(
            r#"
            INSERT INTO documents (
                tenant_id, name, filename, size_bytes, mime_type,
                hash, storage_path, uploaded_by, category_id, tags,
                description, current_version, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, 1, NOW(), NOW())
            RETURNING *
            "#,
        )
        .bind(doc.tenant_id)
        .bind(&doc.name)
        .bind(&doc.filename)
        .bind(doc.size_bytes)
        .bind(&doc.mime_type)
        .bind(&doc.hash)
        .bind(&doc.storage_path)
        .bind(doc.uploaded_by)
        .bind(doc.category_id)
        .bind(doc.tags.unwrap_or_default())
        .bind(&doc.description)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Document"))?;

        Ok(row.into())
    }

    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<Document>, ApiError> {
        let row = sqlx::query_as::<_, DocumentRow>(
            r#"
            SELECT * FROM documents
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NULL
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Document"))?;

        Ok(row.map(Into::into))
    }

    async fn search(
        &self,
        tenant_id: i64,
        params: DocumentSearchParams,
    ) -> Result<DocumentSearchResult, ApiError> {
        let _conditions = ["tenant_id = $1", "deleted_at IS NULL"];
        let _binds: Vec<Box<dyn std::any::Any + Send + Sync>> = Vec::new();
        // Note: dynamic conditions with sqlx are limited; we use a simplified approach
        // that checks all common filters in the query and applies entity filtering in code.

        let base_query = r#"
            SELECT * FROM documents
            WHERE tenant_id = $1 AND deleted_at IS NULL
              AND ($2::text IS NULL OR name ILIKE '%' || $2 || '%'
                   OR filename ILIKE '%' || $2 || '%'
                   OR mime_type ILIKE '%' || $2 || '%'
                   OR tags @> ARRAY[$2])
              AND ($3::bigint IS NULL OR category_id = $3)
              AND ($4::text IS NULL OR mime_type = $4)
              AND ($5::bigint IS NULL OR uploaded_by = $5)
            ORDER BY updated_at DESC
            LIMIT $6 OFFSET $7
        "#;

        let offset = ((params.page.saturating_sub(1)) * params.per_page) as i64;

        let rows = sqlx::query_as::<_, DocumentRow>(base_query)
            .bind(tenant_id)
            .bind(params.query.as_ref())
            .bind(params.category_id)
            .bind(params.mime_type.as_ref())
            .bind(params.uploaded_by)
            .bind(params.per_page as i64)
            .bind(offset)
            .fetch_all(&*self.pool)
            .await
            .map_err(|e| map_sqlx_error(e, "Document"))?;

        let total_query = r#"
            SELECT COUNT(*) FROM documents
            WHERE tenant_id = $1 AND deleted_at IS NULL
              AND ($2::text IS NULL OR name ILIKE '%' || $2 || '%'
                   OR filename ILIKE '%' || $2 || '%'
                   OR mime_type ILIKE '%' || $2 || '%'
                   OR tags @> ARRAY[$2])
              AND ($3::bigint IS NULL OR category_id = $3)
              AND ($4::text IS NULL OR mime_type = $4)
              AND ($5::bigint IS NULL OR uploaded_by = $5)
        "#;

        let total: i64 = sqlx::query_scalar(total_query)
            .bind(tenant_id)
            .bind(params.query.as_ref())
            .bind(params.category_id)
            .bind(params.mime_type.as_ref())
            .bind(params.uploaded_by)
            .fetch_one(&*self.pool)
            .await
            .map_err(|e| map_sqlx_error(e, "Document"))?;

        let mut items: Vec<Document> = rows.into_iter().map(Into::into).collect();

        // Filter by tags in code (array contains check is complex in raw SQL)
        if let Some(ref tags) = params.tags {
            items.retain(|d| tags.iter().all(|t| d.tags.contains(t)));
        }

        // Filter by entity link in code
        if let (Some(ref entity_type), Some(entity_id)) = (&params.entity_type, params.entity_id) {
            let link_rows = sqlx::query_as::<_, DocumentLinkRow>(
                "SELECT * FROM document_links WHERE tenant_id = $1 AND entity_type = $2 AND entity_id = $3"
            )
            .bind(tenant_id)
            .bind(entity_type)
            .bind(entity_id)
            .fetch_all(&*self.pool)
            .await
            .map_err(|e| map_sqlx_error(e, "DocumentLink"))?;

            let linked_ids: Vec<i64> = link_rows.into_iter().map(|l| l.document_id).collect();
            items.retain(|d| linked_ids.contains(&d.id));
        }

        Ok(DocumentSearchResult {
            items: items.into_iter().map(Into::into).collect(),
            total: total as u32,
            page: params.page,
            per_page: params.per_page,
        })
    }

    async fn update(
        &self,
        id: i64,
        tenant_id: i64,
        update: UpdateDocument,
    ) -> Result<Document, ApiError> {
        let row = sqlx::query_as::<_, DocumentRow>(
            r#"
            UPDATE documents
            SET
                name = COALESCE($3, name),
                category_id = COALESCE($4, category_id),
                tags = COALESCE($5, tags),
                description = COALESCE($6, description),
                updated_at = NOW()
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NULL
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .bind(update.name)
        .bind(update.category_id)
        .bind(update.tags)
        .bind(update.description)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Document"))?;

        match row {
            Some(r) => Ok(r.into()),
            None => Err(ApiError::NotFound(format!("Document {} not found", id))),
        }
    }

    async fn soft_delete(&self, id: i64, tenant_id: i64, deleted_by: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            UPDATE documents
            SET deleted_at = NOW(), deleted_by = $3
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NULL
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .bind(deleted_by)
        .execute(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Document"))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound(format!("Document {} not found", id)));
        }
        Ok(())
    }

    async fn restore(&self, id: i64, tenant_id: i64) -> Result<Document, ApiError> {
        let row = sqlx::query_as::<_, DocumentRow>(
            r#"
            UPDATE documents
            SET deleted_at = NULL, deleted_by = NULL
            WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NOT NULL
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Document"))?;

        match row {
            Some(r) => Ok(r.into()),
            None => Err(ApiError::NotFound(format!("Document {} not found", id))),
        }
    }

    async fn find_deleted(&self, tenant_id: i64) -> Result<Vec<Document>, ApiError> {
        let rows = sqlx::query_as::<_, DocumentRow>(
            r#"
            SELECT * FROM documents
            WHERE tenant_id = $1 AND deleted_at IS NOT NULL
            ORDER BY updated_at DESC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Document"))?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn destroy(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| map_sqlx_error(e, "Document"))?;

        sqlx::query("DELETE FROM document_versions WHERE document_id = $1 AND tenant_id = $2")
            .bind(id)
            .bind(tenant_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| map_sqlx_error(e, "Document"))?;

        sqlx::query("DELETE FROM document_links WHERE document_id = $1 AND tenant_id = $2")
            .bind(id)
            .bind(tenant_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| map_sqlx_error(e, "Document"))?;

        let result = sqlx::query("DELETE FROM documents WHERE id = $1 AND tenant_id = $2")
            .bind(id)
            .bind(tenant_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| map_sqlx_error(e, "Document"))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound(format!("Document {} not found", id)));
        }

        tx.commit()
            .await
            .map_err(|e| map_sqlx_error(e, "Document"))?;
        Ok(())
    }

    // --- Versioning ---

    async fn create_version(
        &self,
        version: CreateDocumentVersion,
    ) -> Result<DocumentVersion, ApiError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| map_sqlx_error(e, "Document"))?;

        // Increment current_version
        let updated: Option<(i32,)> = sqlx::query_as(
            "UPDATE documents SET current_version = current_version + 1, updated_at = NOW() WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NULL RETURNING current_version"
        )
        .bind(version.document_id)
        .bind(version.tenant_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| map_sqlx_error(e, "Document"))?;

        let new_version_number = match updated {
            Some((v,)) => v,
            None => {
                return Err(ApiError::NotFound(format!(
                    "Document {} not found",
                    version.document_id
                )))
            }
        };

        let row = sqlx::query_as::<_, DocumentVersionRow>(
            r#"
            INSERT INTO document_versions (
                document_id, tenant_id, version_number, filename,
                size_bytes, hash, storage_path, created_by, comment, created_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, NOW())
            RETURNING *
            "#,
        )
        .bind(version.document_id)
        .bind(version.tenant_id)
        .bind(new_version_number)
        .bind(&version.filename)
        .bind(version.size_bytes)
        .bind(&version.hash)
        .bind(&version.storage_path)
        .bind(version.created_by)
        .bind(&version.comment)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| map_sqlx_error(e, "Document"))?;

        tx.commit()
            .await
            .map_err(|e| map_sqlx_error(e, "Document"))?;

        Ok(row.into())
    }

    async fn list_versions(
        &self,
        document_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<DocumentVersion>, ApiError> {
        let rows = sqlx::query_as::<_, DocumentVersionRow>(
            r#"
            SELECT * FROM document_versions
            WHERE document_id = $1 AND tenant_id = $2
            ORDER BY version_number DESC
            "#,
        )
        .bind(document_id)
        .bind(tenant_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Document"))?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn get_version(
        &self,
        version_id: i64,
        tenant_id: i64,
    ) -> Result<Option<DocumentVersion>, ApiError> {
        let row = sqlx::query_as::<_, DocumentVersionRow>(
            r#"
            SELECT * FROM document_versions
            WHERE id = $1 AND tenant_id = $2
            "#,
        )
        .bind(version_id)
        .bind(tenant_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Document"))?;

        Ok(row.map(Into::into))
    }

    // --- Categories ---

    async fn create_category(
        &self,
        category: CreateDocumentCategory,
    ) -> Result<DocumentCategory, ApiError> {
        let row = sqlx::query_as::<_, DocumentCategoryRow>(
            r#"
            INSERT INTO document_categories (tenant_id, name, description, color, parent_id, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, NOW(), NOW())
            RETURNING *
            "#,
        )
        .bind(category.tenant_id)
        .bind(&category.name)
        .bind(&category.description)
        .bind(&category.color)
        .bind(category.parent_id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Document"))?;

        Ok(row.into())
    }

    async fn find_category_by_id(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<Option<DocumentCategory>, ApiError> {
        let row = sqlx::query_as::<_, DocumentCategoryRow>(
            "SELECT * FROM document_categories WHERE id = $1 AND tenant_id = $2",
        )
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Document"))?;

        Ok(row.map(Into::into))
    }

    async fn list_categories(&self, tenant_id: i64) -> Result<Vec<DocumentCategory>, ApiError> {
        let rows = sqlx::query_as::<_, DocumentCategoryRow>(
            "SELECT * FROM document_categories WHERE tenant_id = $1 ORDER BY name",
        )
        .bind(tenant_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Document"))?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn update_category(
        &self,
        id: i64,
        tenant_id: i64,
        update: UpdateDocumentCategory,
    ) -> Result<DocumentCategory, ApiError> {
        let row = sqlx::query_as::<_, DocumentCategoryRow>(
            r#"
            UPDATE document_categories
            SET
                name = COALESCE($3, name),
                description = COALESCE($4, description),
                color = COALESCE($5, color),
                parent_id = COALESCE($6, parent_id),
                updated_at = NOW()
            WHERE id = $1 AND tenant_id = $2
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .bind(update.name)
        .bind(update.description)
        .bind(update.color)
        .bind(update.parent_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Document"))?;

        match row {
            Some(r) => Ok(r.into()),
            None => Err(ApiError::NotFound(format!("Category {} not found", id))),
        }
    }

    async fn delete_category(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| map_sqlx_error(e, "Document"))?;

        sqlx::query(
            "UPDATE documents SET category_id = NULL WHERE category_id = $1 AND tenant_id = $2",
        )
        .bind(id)
        .bind(tenant_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| map_sqlx_error(e, "Document"))?;

        let result =
            sqlx::query("DELETE FROM document_categories WHERE id = $1 AND tenant_id = $2")
                .bind(id)
                .bind(tenant_id)
                .execute(&mut *tx)
                .await
                .map_err(|e| map_sqlx_error(e, "Document"))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound(format!("Category {} not found", id)));
        }

        tx.commit()
            .await
            .map_err(|e| map_sqlx_error(e, "Document"))?;
        Ok(())
    }

    // --- Entity Links ---

    async fn create_link(&self, link: CreateDocumentLink) -> Result<DocumentLink, ApiError> {
        let row = sqlx::query_as::<_, DocumentLinkRow>(
            r#"
            INSERT INTO document_links (document_id, tenant_id, entity_type, entity_id, created_at)
            VALUES ($1, $2, $3, $4, NOW())
            RETURNING *
            "#,
        )
        .bind(link.document_id)
        .bind(link.tenant_id)
        .bind(&link.entity_type)
        .bind(link.entity_id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Document"))?;

        Ok(row.into())
    }

    async fn delete_link(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let result = sqlx::query("DELETE FROM document_links WHERE id = $1 AND tenant_id = $2")
            .bind(id)
            .bind(tenant_id)
            .execute(&*self.pool)
            .await
            .map_err(|e| map_sqlx_error(e, "Document"))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound(format!("Link {} not found", id)));
        }
        Ok(())
    }

    async fn list_links_by_document(
        &self,
        document_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<DocumentLink>, ApiError> {
        let rows = sqlx::query_as::<_, DocumentLinkRow>(
            "SELECT * FROM document_links WHERE document_id = $1 AND tenant_id = $2 ORDER BY created_at DESC",
        )
        .bind(document_id)
        .bind(tenant_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Document"))?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn list_links_by_entity(
        &self,
        tenant_id: i64,
        entity_type: &str,
        entity_id: i64,
    ) -> Result<Vec<DocumentLink>, ApiError> {
        let rows = sqlx::query_as::<_, DocumentLinkRow>(
            "SELECT * FROM document_links WHERE tenant_id = $1 AND entity_type = $2 AND entity_id = $3 ORDER BY created_at DESC",
        )
        .bind(tenant_id)
        .bind(entity_type)
        .bind(entity_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Document"))?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn find_documents_by_entity(
        &self,
        tenant_id: i64,
        entity_type: &str,
        entity_id: i64,
    ) -> Result<Vec<Document>, ApiError> {
        let rows = sqlx::query_as::<_, DocumentRow>(
            r#"
            SELECT d.* FROM documents d
            JOIN document_links l ON d.id = l.document_id
            WHERE l.tenant_id = $1
              AND l.entity_type = $2
              AND l.entity_id = $3
              AND d.deleted_at IS NULL
            ORDER BY d.updated_at DESC
            "#,
        )
        .bind(tenant_id)
        .bind(entity_type)
        .bind(entity_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Document"))?;

        Ok(rows.into_iter().map(Into::into).collect())
    }
}
