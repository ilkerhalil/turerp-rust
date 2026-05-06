//! Custom field repository trait and in-memory implementation

use async_trait::async_trait;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::Arc;

use crate::common::SoftDeletable;
use crate::domain::custom_field::model::{CustomFieldDefinition, CustomFieldModule};
use crate::error::ApiError;

/// Repository trait for custom field definition operations
#[allow(clippy::too_many_arguments)]
#[async_trait]
pub trait CustomFieldRepository: Send + Sync {
    async fn create(&self, def: CustomFieldDefinition) -> Result<CustomFieldDefinition, ApiError>;

    async fn find_by_id(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<Option<CustomFieldDefinition>, ApiError>;

    async fn find_by_module(
        &self,
        tenant_id: i64,
        module: CustomFieldModule,
    ) -> Result<Vec<CustomFieldDefinition>, ApiError>;

    async fn find_all(&self, tenant_id: i64) -> Result<Vec<CustomFieldDefinition>, ApiError>;

    async fn field_name_exists(
        &self,
        tenant_id: i64,
        module: CustomFieldModule,
        field_name: &str,
    ) -> Result<bool, ApiError>;

    async fn update(
        &self,
        id: i64,
        tenant_id: i64,
        label: Option<String>,
        required: Option<bool>,
        options: Option<Vec<String>>,
        sort_order: Option<i32>,
        is_active: Option<bool>,
    ) -> Result<CustomFieldDefinition, ApiError>;

    async fn soft_delete(&self, id: i64, tenant_id: i64, deleted_by: i64) -> Result<(), ApiError>;

    async fn restore(&self, id: i64, tenant_id: i64) -> Result<(), ApiError>;

    async fn find_deleted(&self, tenant_id: i64) -> Result<Vec<CustomFieldDefinition>, ApiError>;

    async fn destroy(&self, id: i64, tenant_id: i64) -> Result<(), ApiError>;
}

/// Type alias for boxed repository
pub type BoxCustomFieldRepository = Arc<dyn CustomFieldRepository>;

/// Inner state for InMemoryCustomFieldRepository
struct InMemoryCustomFieldInner {
    definitions: HashMap<i64, CustomFieldDefinition>,
    next_id: i64,
}

/// In-memory custom field repository
pub struct InMemoryCustomFieldRepository {
    inner: Mutex<InMemoryCustomFieldInner>,
}

impl InMemoryCustomFieldRepository {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(InMemoryCustomFieldInner {
                definitions: HashMap::new(),
                next_id: 1,
            }),
        }
    }
}

impl Default for InMemoryCustomFieldRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CustomFieldRepository for InMemoryCustomFieldRepository {
    async fn create(&self, def: CustomFieldDefinition) -> Result<CustomFieldDefinition, ApiError> {
        let mut inner = self.inner.lock();
        let id = inner.next_id;
        inner.next_id += 1;

        let new_def = CustomFieldDefinition { id, ..def };
        inner.definitions.insert(id, new_def.clone());
        Ok(new_def)
    }

    async fn find_by_id(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<Option<CustomFieldDefinition>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .definitions
            .get(&id)
            .filter(|d| d.tenant_id == tenant_id && !d.is_deleted())
            .cloned())
    }

    async fn find_by_module(
        &self,
        tenant_id: i64,
        module: CustomFieldModule,
    ) -> Result<Vec<CustomFieldDefinition>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .definitions
            .values()
            .filter(|d| {
                d.tenant_id == tenant_id && d.module == module && !d.is_deleted() && d.is_active
            })
            .cloned()
            .collect())
    }

    async fn find_all(&self, tenant_id: i64) -> Result<Vec<CustomFieldDefinition>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .definitions
            .values()
            .filter(|d| d.tenant_id == tenant_id && !d.is_deleted())
            .cloned()
            .collect())
    }

    async fn field_name_exists(
        &self,
        tenant_id: i64,
        module: CustomFieldModule,
        field_name: &str,
    ) -> Result<bool, ApiError> {
        let inner = self.inner.lock();
        Ok(inner.definitions.values().any(|d| {
            d.tenant_id == tenant_id
                && d.module == module
                && d.field_name == field_name
                && !d.is_deleted()
        }))
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
        let mut inner = self.inner.lock();

        let def = inner
            .definitions
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound(format!("Custom field {} not found", id)))?;

        if def.tenant_id != tenant_id {
            return Err(ApiError::NotFound(format!("Custom field {} not found", id)));
        }

        if let Some(l) = label {
            def.field_label = l;
        }
        if let Some(r) = required {
            def.required = r;
        }
        if let Some(o) = options {
            def.options = o;
        }
        if let Some(s) = sort_order {
            def.sort_order = s;
        }
        if let Some(a) = is_active {
            def.is_active = a;
        }

        def.updated_at = Some(chrono::Utc::now());
        Ok(def.clone())
    }

    async fn soft_delete(&self, id: i64, tenant_id: i64, deleted_by: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();

        let def = inner
            .definitions
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound(format!("Custom field {} not found", id)))?;

        if def.tenant_id != tenant_id {
            return Err(ApiError::NotFound(format!("Custom field {} not found", id)));
        }

        def.mark_deleted(deleted_by);
        Ok(())
    }

    async fn restore(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();

        let def = inner
            .definitions
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound(format!("Custom field {} not found", id)))?;

        if def.tenant_id != tenant_id {
            return Err(ApiError::NotFound(format!("Custom field {} not found", id)));
        }

        if !def.is_deleted() {
            return Err(ApiError::BadRequest(
                "Custom field is not deleted".to_string(),
            ));
        }

        def.restore();
        Ok(())
    }

    async fn find_deleted(&self, tenant_id: i64) -> Result<Vec<CustomFieldDefinition>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .definitions
            .values()
            .filter(|d| d.tenant_id == tenant_id && d.is_deleted())
            .cloned()
            .collect())
    }

    async fn destroy(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();

        let def = inner
            .definitions
            .get(&id)
            .ok_or_else(|| ApiError::NotFound(format!("Custom field {} not found", id)))?;

        if def.tenant_id != tenant_id {
            return Err(ApiError::NotFound(format!("Custom field {} not found", id)));
        }

        if !def.is_deleted() {
            return Err(ApiError::BadRequest(
                "Custom field must be soft deleted before destroy".to_string(),
            ));
        }

        inner.definitions.remove(&id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::custom_field::model::CustomFieldType;

    fn create_repo() -> InMemoryCustomFieldRepository {
        InMemoryCustomFieldRepository::new()
    }

    fn make_definition(
        tenant_id: i64,
        module: CustomFieldModule,
        name: &str,
    ) -> CustomFieldDefinition {
        CustomFieldDefinition {
            id: 0,
            tenant_id,
            module,
            field_name: name.to_string(),
            field_label: name.to_string(),
            field_type: CustomFieldType::String,
            required: false,
            options: vec![],
            sort_order: 0,
            is_active: true,
            created_at: chrono::Utc::now(),
            updated_at: None,
            deleted_at: None,
            deleted_by: None,
        }
    }

    #[tokio::test]
    async fn test_create_and_find() {
        let repo = create_repo();
        let def = make_definition(1, CustomFieldModule::Cari, "tax_region");
        let created = repo.create(def).await.unwrap();
        assert_eq!(created.id, 1);

        let found = repo.find_by_id(1, 1).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().field_name, "tax_region");
    }

    #[tokio::test]
    async fn test_find_by_module() {
        let repo = create_repo();
        repo.create(make_definition(1, CustomFieldModule::Cari, "f1"))
            .await
            .unwrap();
        repo.create(make_definition(1, CustomFieldModule::Invoice, "f2"))
            .await
            .unwrap();
        repo.create(make_definition(1, CustomFieldModule::Cari, "f3"))
            .await
            .unwrap();

        let cari_fields = repo
            .find_by_module(1, CustomFieldModule::Cari)
            .await
            .unwrap();
        assert_eq!(cari_fields.len(), 2);
    }

    #[tokio::test]
    async fn test_field_name_exists() {
        let repo = create_repo();
        repo.create(make_definition(1, CustomFieldModule::Cari, "tax_region"))
            .await
            .unwrap();

        assert!(repo
            .field_name_exists(1, CustomFieldModule::Cari, "tax_region")
            .await
            .unwrap());
        assert!(!repo
            .field_name_exists(1, CustomFieldModule::Cari, "other")
            .await
            .unwrap());
        assert!(!repo
            .field_name_exists(2, CustomFieldModule::Cari, "tax_region")
            .await
            .unwrap());
    }

    #[tokio::test]
    async fn test_update() {
        let repo = create_repo();
        repo.create(make_definition(1, CustomFieldModule::Cari, "f1"))
            .await
            .unwrap();

        let updated = repo
            .update(
                1,
                1,
                Some("New Label".to_string()),
                Some(true),
                None,
                None,
                None,
            )
            .await
            .unwrap();
        assert_eq!(updated.field_label, "New Label");
        assert!(updated.required);
    }

    #[tokio::test]
    async fn test_soft_delete() {
        let repo = create_repo();
        repo.create(make_definition(1, CustomFieldModule::Cari, "f1"))
            .await
            .unwrap();

        repo.soft_delete(1, 1, 42).await.unwrap();

        let found = repo.find_by_id(1, 1).await.unwrap();
        assert!(found.is_none());

        let cari_fields = repo
            .find_by_module(1, CustomFieldModule::Cari)
            .await
            .unwrap();
        assert!(cari_fields.is_empty());
    }

    #[tokio::test]
    async fn test_tenant_isolation() {
        let repo = create_repo();
        repo.create(make_definition(1, CustomFieldModule::Cari, "f1"))
            .await
            .unwrap();

        let found = repo.find_by_id(1, 999).await.unwrap();
        assert!(found.is_none());
    }
}
