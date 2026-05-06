//! Custom field service with validation logic

use validator::Validate;

use crate::domain::custom_field::model::{
    CreateCustomFieldDefinition, CustomFieldDefinition, CustomFieldDefinitionResponse,
    CustomFieldModule, CustomFieldType, CustomFieldValues, UpdateCustomFieldDefinition,
};
use crate::domain::custom_field::repository::BoxCustomFieldRepository;
use crate::error::ApiError;

/// Custom field service
#[derive(Clone)]
pub struct CustomFieldService {
    repo: BoxCustomFieldRepository,
}

impl CustomFieldService {
    pub fn new(repo: BoxCustomFieldRepository) -> Self {
        Self { repo }
    }

    /// Create a new custom field definition
    pub async fn create(
        &self,
        create: CreateCustomFieldDefinition,
    ) -> Result<CustomFieldDefinitionResponse, ApiError> {
        create
            .validate()
            .map_err(|e| ApiError::Validation(e.to_string()))?;

        let module: CustomFieldModule = create
            .module
            .parse()
            .map_err(|e: String| ApiError::Validation(e))?;

        let field_type: CustomFieldType = create
            .field_type
            .parse()
            .map_err(|e: String| ApiError::Validation(e))?;

        if field_type == CustomFieldType::Select && create.options.is_empty() {
            return Err(ApiError::Validation(
                "Select type fields must have at least one option".to_string(),
            ));
        }

        if self
            .repo
            .field_name_exists(create.tenant_id, module, &create.field_name)
            .await?
        {
            return Err(ApiError::Conflict(format!(
                "Custom field '{}' already exists in module '{}'",
                create.field_name, module
            )));
        }

        let def = CustomFieldDefinition {
            id: 0,
            tenant_id: create.tenant_id,
            module,
            field_name: create.field_name,
            field_label: create.field_label,
            field_type,
            required: create.required,
            options: create.options,
            sort_order: create.sort_order,
            is_active: true,
            created_at: chrono::Utc::now(),
            updated_at: None,
            deleted_at: None,
            deleted_by: None,
        };

        let created = self.repo.create(def).await?;
        Ok(created.into())
    }

    /// Get a custom field definition by ID
    pub async fn get_by_id(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<CustomFieldDefinitionResponse, ApiError> {
        let def = self
            .repo
            .find_by_id(id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Custom field {} not found", id)))?;
        Ok(def.into())
    }

    /// List custom field definitions by module
    pub async fn list_by_module(
        &self,
        tenant_id: i64,
        module: CustomFieldModule,
    ) -> Result<Vec<CustomFieldDefinitionResponse>, ApiError> {
        let defs = self.repo.find_by_module(tenant_id, module).await?;
        Ok(defs.into_iter().map(|d| d.into()).collect())
    }

    /// List all custom field definitions for a tenant
    pub async fn list_all(
        &self,
        tenant_id: i64,
    ) -> Result<Vec<CustomFieldDefinitionResponse>, ApiError> {
        let defs = self.repo.find_all(tenant_id).await?;
        Ok(defs.into_iter().map(|d| d.into()).collect())
    }

    /// Update a custom field definition
    pub async fn update(
        &self,
        id: i64,
        tenant_id: i64,
        update: UpdateCustomFieldDefinition,
    ) -> Result<CustomFieldDefinitionResponse, ApiError> {
        update
            .validate()
            .map_err(|e| ApiError::Validation(e.to_string()))?;

        if let Some(ref options) = update.options {
            let existing = self.repo.find_by_id(id, tenant_id).await?;
            let def = existing
                .ok_or_else(|| ApiError::NotFound(format!("Custom field {} not found", id)))?;
            if def.field_type == CustomFieldType::Select && options.is_empty() {
                return Err(ApiError::Validation(
                    "Select type fields must have at least one option".to_string(),
                ));
            }
        }

        let updated = self
            .repo
            .update(
                id,
                tenant_id,
                update.field_label,
                update.required,
                update.options,
                update.sort_order,
                update.is_active,
            )
            .await?;
        Ok(updated.into())
    }

    /// Soft delete a custom field definition
    pub async fn soft_delete(
        &self,
        id: i64,
        tenant_id: i64,
        deleted_by: i64,
    ) -> Result<(), ApiError> {
        self.repo.soft_delete(id, tenant_id, deleted_by).await
    }

    /// Restore a soft-deleted custom field definition
    pub async fn restore(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        self.repo.restore(id, tenant_id).await
    }

    /// List deleted custom field definitions for a tenant
    pub async fn list_deleted(
        &self,
        tenant_id: i64,
    ) -> Result<Vec<CustomFieldDefinitionResponse>, ApiError> {
        let defs = self.repo.find_deleted(tenant_id).await?;
        Ok(defs.into_iter().map(|d| d.into()).collect())
    }

    /// Permanently destroy a soft-deleted custom field definition
    pub async fn destroy(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        self.repo.destroy(id, tenant_id).await
    }

    /// Validate custom field values against their definitions
    pub async fn validate_entity_custom_fields(
        &self,
        tenant_id: i64,
        module: CustomFieldModule,
        values: &CustomFieldValues,
    ) -> Result<(), ApiError> {
        let definitions = self.repo.find_by_module(tenant_id, module).await?;
        validate_custom_fields(&definitions, values)
    }
}

/// Validate custom field values against definitions (standalone, no repo needed)
pub fn validate_custom_fields(
    definitions: &[CustomFieldDefinition],
    values: &CustomFieldValues,
) -> Result<(), ApiError> {
    let obj = match values.as_object() {
        Some(o) => o,
        None => {
            return Err(ApiError::Validation(
                "Custom field values must be a JSON object".to_string(),
            ));
        }
    };

    for def in definitions {
        if !def.is_active {
            continue;
        }

        let value = obj.get(&def.field_name);

        match (value, def.required) {
            (None, true) => {
                return Err(ApiError::Validation(format!(
                    "Required custom field '{}' is missing",
                    def.field_name
                )));
            }
            (None, false) => continue,
            (Some(v), _) => {
                validate_field_type(&def.field_name, def.field_type, v, &def.options)?;
            }
        }
    }

    Ok(())
}

/// Validate a single field value against its type definition
fn validate_field_type(
    name: &str,
    field_type: CustomFieldType,
    value: &serde_json::Value,
    allowed_options: &[String],
) -> Result<(), ApiError> {
    let type_ok = match field_type {
        CustomFieldType::String => value.is_string(),
        CustomFieldType::Number => value.is_number(),
        CustomFieldType::Date => value.is_string(),
        CustomFieldType::Boolean => value.is_boolean(),
        CustomFieldType::Select => value
            .as_str()
            .map(|s| allowed_options.contains(&s.to_string()))
            .unwrap_or(false),
    };

    if !type_ok {
        return Err(ApiError::Validation(format!(
            "Custom field '{}' expects type '{}', got invalid value",
            name, field_type
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::custom_field::repository::InMemoryCustomFieldRepository;
    use std::sync::Arc;

    fn create_service() -> CustomFieldService {
        let repo = Arc::new(InMemoryCustomFieldRepository::new()) as BoxCustomFieldRepository;
        CustomFieldService::new(repo)
    }

    fn make_create_dto(module: &str, name: &str, field_type: &str) -> CreateCustomFieldDefinition {
        CreateCustomFieldDefinition {
            module: module.to_string(),
            field_name: name.to_string(),
            field_label: name.to_string(),
            field_type: field_type.to_string(),
            required: false,
            options: vec![],
            sort_order: 0,
            tenant_id: 1,
        }
    }

    #[tokio::test]
    async fn test_create_custom_field() {
        let service = create_service();
        let dto = make_create_dto("cari", "tax_region", "string");
        let result = service.create(dto).await.unwrap();
        assert_eq!(result.field_name, "tax_region");
        assert_eq!(result.module, CustomFieldModule::Cari);
    }

    #[tokio::test]
    async fn test_create_duplicate_fails() {
        let service = create_service();
        let dto = make_create_dto("cari", "tax_region", "string");
        service.create(dto.clone()).await.unwrap();
        let result = service.create(dto).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ApiError::Conflict(_)));
    }

    #[tokio::test]
    async fn test_create_select_without_options_fails() {
        let service = create_service();
        let mut dto = make_create_dto("cari", "industry", "select");
        dto.options = vec![];
        let result = service.create(dto).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_list_by_module() {
        let service = create_service();
        service
            .create(make_create_dto("cari", "f1", "string"))
            .await
            .unwrap();
        service
            .create(make_create_dto("invoice", "f2", "number"))
            .await
            .unwrap();
        service
            .create(make_create_dto("cari", "f3", "boolean"))
            .await
            .unwrap();

        let cari_fields = service
            .list_by_module(1, CustomFieldModule::Cari)
            .await
            .unwrap();
        assert_eq!(cari_fields.len(), 2);
    }

    #[tokio::test]
    async fn test_soft_delete() {
        let service = create_service();
        let created = service
            .create(make_create_dto("cari", "f1", "string"))
            .await
            .unwrap();

        service.soft_delete(created.id, 1, 42).await.unwrap();

        let result = service.get_by_id(created.id, 1).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_string_field() {
        let defs = vec![CustomFieldDefinition {
            id: 1,
            tenant_id: 1,
            module: CustomFieldModule::Cari,
            field_name: "region".to_string(),
            field_label: "Region".to_string(),
            field_type: CustomFieldType::String,
            required: true,
            options: vec![],
            sort_order: 0,
            is_active: true,
            created_at: chrono::Utc::now(),
            updated_at: None,
            deleted_at: None,
            deleted_by: None,
        }];

        let values = serde_json::json!({"region": "Marmara"});
        assert!(validate_custom_fields(&defs, &values).is_ok());

        let missing = serde_json::json!({});
        assert!(validate_custom_fields(&defs, &missing).is_err());

        let wrong_type = serde_json::json!({"region": 42});
        assert!(validate_custom_fields(&defs, &wrong_type).is_err());
    }

    #[test]
    fn test_validate_select_field() {
        let defs = vec![CustomFieldDefinition {
            id: 1,
            tenant_id: 1,
            module: CustomFieldModule::Cari,
            field_name: "industry".to_string(),
            field_label: "Industry".to_string(),
            field_type: CustomFieldType::Select,
            required: false,
            options: vec!["Tech".to_string(), "Finance".to_string()],
            sort_order: 0,
            is_active: true,
            created_at: chrono::Utc::now(),
            updated_at: None,
            deleted_at: None,
            deleted_by: None,
        }];

        let valid = serde_json::json!({"industry": "Tech"});
        assert!(validate_custom_fields(&defs, &valid).is_ok());

        let invalid_option = serde_json::json!({"industry": "Healthcare"});
        assert!(validate_custom_fields(&defs, &invalid_option).is_err());
    }

    #[test]
    fn test_validate_non_object_fails() {
        let defs: Vec<CustomFieldDefinition> = vec![];
        let values = serde_json::json!("not an object");
        assert!(validate_custom_fields(&defs, &values).is_err());
    }
}
