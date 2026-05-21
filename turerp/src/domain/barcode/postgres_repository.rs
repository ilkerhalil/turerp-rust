//! PostgreSQL barcode repository implementation

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};
use std::sync::Arc;

use crate::common::pagination::PaginatedResult;
use crate::db::error::map_sqlx_error;
use crate::domain::barcode::model::{BarcodeConfig, BarcodeType, CreateBarcode};
use crate::domain::barcode::repository::BarcodeRepository;
use crate::error::ApiError;

/// Database row representation for BarcodeConfig
#[derive(Debug, FromRow)]
struct BarcodeConfigRow {
    id: i64,
    tenant_id: i64,
    entity_type: String,
    entity_id: i64,
    barcode_type: String,
    code: String,
    image_data: Option<String>,
    created_at: DateTime<Utc>,
    total_count: Option<i64>,
}

impl From<BarcodeConfigRow> for BarcodeConfig {
    fn from(row: BarcodeConfigRow) -> Self {
        Self {
            id: row.id,
            tenant_id: row.tenant_id,
            entity_type: row.entity_type,
            entity_id: row.entity_id,
            barcode_type: row.barcode_type.parse().unwrap_or_else(|e| {
                tracing::warn!(error = %e, "Invalid barcode type in database");
                BarcodeType::default()
            }),
            code: row.code,
            image_data: row.image_data,
            created_at: row.created_at,
        }
    }
}

/// PostgreSQL barcode repository
pub struct PostgresBarcodeRepository {
    pool: Arc<PgPool>,
}

impl PostgresBarcodeRepository {
    /// Create a new PostgreSQL barcode repository
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    /// Convert to boxed trait object
    pub fn into_boxed(self) -> Arc<dyn BarcodeRepository> {
        Arc::new(self) as Arc<dyn BarcodeRepository>
    }
}

#[async_trait]
impl BarcodeRepository for PostgresBarcodeRepository {
    async fn find_by_entity(
        &self,
        tenant_id: i64,
        entity_type: &str,
        entity_id: i64,
    ) -> Result<Option<BarcodeConfig>, ApiError> {
        let result: Option<BarcodeConfigRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, entity_type, entity_id, barcode_type, code, image_data, created_at, NULL::bigint AS total_count
            FROM barcode_configs
            WHERE tenant_id = $1 AND entity_type = $2 AND entity_id = $3
            "#,
        )
        .bind(tenant_id)
        .bind(entity_type)
        .bind(entity_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Barcode"))?;

        Ok(result.map(|r| r.into()))
    }

    async fn find_by_tenant(
        &self,
        tenant_id: i64,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<BarcodeConfig>, ApiError> {
        let offset = (page.saturating_sub(1)) * per_page;

        let rows: Vec<BarcodeConfigRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, entity_type, entity_id, barcode_type, code, image_data, created_at,
                   COUNT(*) OVER() as total_count
            FROM barcode_configs
            WHERE tenant_id = $1
            ORDER BY id DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(tenant_id)
        .bind(per_page as i64)
        .bind(offset as i64)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Barcode"))?;

        let total = rows.first().and_then(|r| r.total_count).unwrap_or(0) as u64;
        let items: Vec<BarcodeConfig> = rows.into_iter().map(|r| r.into()).collect();
        Ok(PaginatedResult::new(items, page, per_page, total))
    }

    async fn create(
        &self,
        tenant_id: i64,
        create: CreateBarcode,
    ) -> Result<BarcodeConfig, ApiError> {
        let barcode_type_str = create.barcode_type.to_string();

        let row: BarcodeConfigRow = sqlx::query_as(
            r#"
            INSERT INTO barcode_configs (tenant_id, entity_type, entity_id, barcode_type, code, image_data, created_at)
            VALUES ($1, $2, $3, $4, $5, NULL, NOW())
            RETURNING id, tenant_id, entity_type, entity_id, barcode_type, code, image_data, created_at, NULL::bigint AS total_count
            "#,
        )
        .bind(tenant_id)
        .bind(&create.entity_type)
        .bind(create.entity_id)
        .bind(&barcode_type_str)
        .bind(&create.code)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Barcode"))?;

        Ok(row.into())
    }

    async fn delete(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            DELETE FROM barcode_configs
            WHERE id = $1 AND tenant_id = $2
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .execute(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Barcode"))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound(format!("Barcode {} not found", id)));
        }

        Ok(())
    }

    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<BarcodeConfig>, ApiError> {
        let result: Option<BarcodeConfigRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, entity_type, entity_id, barcode_type, code, image_data, created_at, NULL::bigint AS total_count
            FROM barcode_configs
            WHERE id = $1 AND tenant_id = $2
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Barcode"))?;

        Ok(result.map(|r| r.into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_barcode_type_parse_in_from_row() {
        let row = BarcodeConfigRow {
            id: 1,
            tenant_id: 1,
            entity_type: "product".to_string(),
            entity_id: 42,
            barcode_type: "Ean13".to_string(),
            code: "5901234123457".to_string(),
            image_data: None,
            created_at: Utc::now(),
            total_count: None,
        };
        let config: BarcodeConfig = row.into();
        assert_eq!(config.barcode_type, BarcodeType::Ean13);
    }

    #[test]
    fn test_invalid_barcode_type_defaults() {
        let row = BarcodeConfigRow {
            id: 1,
            tenant_id: 1,
            entity_type: "product".to_string(),
            entity_id: 42,
            barcode_type: "UnknownType".to_string(),
            code: "5901234123457".to_string(),
            image_data: None,
            created_at: Utc::now(),
            total_count: None,
        };
        let config: BarcodeConfig = row.into();
        assert_eq!(config.barcode_type, BarcodeType::default());
    }
}
