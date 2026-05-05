//! PostgreSQL implementation of custom field repository

#[cfg(feature = "postgres")]
use async_trait::async_trait;
#[cfg(feature = "postgres")]
use sqlx::PgPool;
#[cfg(feature = "postgres")]
use std::sync::Arc;

#[cfg(feature = "postgres")]
use crate::common::SoftDeletable;
#[cfg(feature = "postgres")]
use crate::domain::custom_field::model::{
    CustomFieldDefinition, CustomFieldModule, CustomFieldType,
};
#[cfg(feature = "postgres")]
use crate::domain::custom_field::repository::CustomFieldRepository;
#[cfg(feature = "postgres")]
use crate::error::ApiError;

/// PostgreSQL custom field repository
#[cfg(feature = "postgres")]
pub struct PostgresCustomFieldRepository {
    pool: Arc<PgPool>,
}

#[cfg(feature = "postgres")]
impl PostgresCustomFieldRepository {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    pub fn into_boxed(self) -> Arc<dyn CustomFieldRepository> {
        Arc::new(self)
    }
}

#[cfg(feature = "postgres")]
#[async_trait]
impl CustomFieldRepository for PostgresCustomFieldRepository {
    async fn create(&self, def: CustomFieldDefinition) -> Result<CustomFieldDefinition, ApiError> {
        let options_json =
            serde_json::to_value(&def.options).map_err(|e| ApiError::Internal(e.to_string()))?;

        let row = sqlx::query_as!(
            CustomFieldDefinitionRow,
            r#"INSERT INTO custom_field_definitions
                (tenant_id, module, field_name, field_label, field_type, required, options, sort_order, is_active)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                RETURNING id, tenant_id, module, field_name, field_label, field_type, required,
                    options as "options: Vec<String>", sort_order, is_active,
                    created_at, updated_at, deleted_at, deleted_by"#,
            def.tenant_id,
            def.module.to_string(),
            def.field_name,
            def.field_label,
            def.field_type.to_string(),
            def.required,
            options_json,
            def.sort_order,
            def.is_active,
        )
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

        Ok(row.into_definition())
    }

    async fn find_by_id(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<Option<CustomFieldDefinition>, ApiError> {
        let result = sqlx::query_as!(
            CustomFieldDefinitionRow,
            r#"SELECT id, tenant_id, module, field_name, field_label, field_type, required,
                options as "options: Vec<String>", sort_order, is_active,
                created_at, updated_at, deleted_at, deleted_by
                FROM custom_field_definitions
                WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NULL"#,
            id,
            tenant_id,
        )
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

        Ok(result.map(|r| r.into_definition()))
    }

    async fn find_by_module(
        &self,
        tenant_id: i64,
        module: CustomFieldModule,
    ) -> Result<Vec<CustomFieldDefinition>, ApiError> {
        let rows = sqlx::query_as!(
            CustomFieldDefinitionRow,
            r#"SELECT id, tenant_id, module, field_name, field_label, field_type, required,
                options as "options: Vec<String>", sort_order, is_active,
                created_at, updated_at, deleted_at, deleted_by
                FROM custom_field_definitions
                WHERE tenant_id = $1 AND module = $2 AND deleted_at IS NULL AND is_active = true
                ORDER BY sort_order"#,
            tenant_id,
            module.to_string(),
        )
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

        Ok(rows.into_iter().map(|r| r.into_definition()).collect())
    }

    async fn find_all(&self, tenant_id: i64) -> Result<Vec<CustomFieldDefinition>, ApiError> {
        let rows = sqlx::query_as!(
            CustomFieldDefinitionRow,
            r#"SELECT id, tenant_id, module, field_name, field_label, field_type, required,
                options as "options: Vec<String>", sort_order, is_active,
                created_at, updated_at, deleted_at, deleted_by
                FROM custom_field_definitions
                WHERE tenant_id = $1 AND deleted_at IS NULL
                ORDER BY module, sort_order"#,
            tenant_id,
        )
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

        Ok(rows.into_iter().map(|r| r.into_definition()).collect())
    }

    async fn field_name_exists(
        &self,
        tenant_id: i64,
        module: CustomFieldModule,
        field_name: &str,
    ) -> Result<bool, ApiError> {
        let result = sqlx::query_scalar!(
            r#"SELECT EXISTS(
                SELECT 1 FROM custom_field_definitions
                WHERE tenant_id = $1 AND module = $2 AND field_name = $3 AND deleted_at IS NULL
            )"#,
            tenant_id,
            module.to_string(),
            field_name,
        )
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

        Ok(result.unwrap_or(false))
    }

    async fn update(
        &self,
        id: i64,
        tenant_id: i64,
        label: Option<String>,
        required: Option<bool>,
        options: Option<Vec<String>>,
        sort_order: Option<i32>,
        is_active: Option<bool>,
    ) -> Result<CustomFieldDefinition, ApiError> {
        let options_json = options
            .map(|o| serde_json::to_value(&o))
            .transpose()
            .map_err(|e| ApiError::Internal(e.to_string()))?;

        let row = sqlx::query_as!(
            CustomFieldDefinitionRow,
            r#"UPDATE custom_field_definitions
                SET field_label = COALESCE($3, field_label),
                    required = COALESCE($4, required),
                    options = COALESCE($5, options),
                    sort_order = COALESCE($6, sort_order),
                    is_active = COALESCE($7, is_active),
                    updated_at = NOW()
                WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NULL
                RETURNING id, tenant_id, module, field_name, field_label, field_type, required,
                    options as "options: Vec<String>", sort_order, is_active,
                    created_at, updated_at, deleted_at, deleted_by"#,
            id,
            tenant_id,
            label,
            required,
            options_json,
            sort_order,
            is_active,
        )
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Custom field {} not found", id)))?;

        Ok(row.into_definition())
    }

    async fn soft_delete(&self, id: i64, tenant_id: i64, deleted_by: i64) -> Result<(), ApiError> {
        let result = sqlx::query!(
            r#"UPDATE custom_field_definitions
                SET deleted_at = NOW(), deleted_by = $3
                WHERE id = $1 AND tenant_id = $2 AND deleted_at IS NULL"#,
            id,
            tenant_id,
            deleted_by,
        )
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound(format!("Custom field {} not found", id)));
        }

        Ok(())
    }
}

/// Helper struct for sqlx mapping
#[cfg(feature = "postgres")]
struct CustomFieldDefinitionRow {
    id: i64,
    tenant_id: i64,
    module: String,
    field_name: String,
    field_label: String,
    field_type: String,
    required: bool,
    options: Vec<String>,
    sort_order: i32,
    is_active: bool,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: Option<chrono::DateTime<chrono::Utc>>,
    deleted_at: Option<chrono::DateTime<chrono::Utc>>,
    deleted_by: Option<i64>,
}

#[cfg(feature = "postgres")]
impl CustomFieldDefinitionRow {
    fn into_definition(self) -> CustomFieldDefinition {
        CustomFieldDefinition {
            id: self.id,
            tenant_id: self.tenant_id,
            module: self.module.parse().unwrap_or(CustomFieldModule::Cari),
            field_name: self.field_name,
            field_label: self.field_label,
            field_type: self.field_type.parse().unwrap_or(CustomFieldType::String),
            required: self.required,
            options: self.options,
            sort_order: self.sort_order,
            is_active: self.is_active,
            created_at: self.created_at,
            updated_at: self.updated_at,
            deleted_at: self.deleted_at,
            deleted_by: self.deleted_by,
        }
    }
}
