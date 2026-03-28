//! Product service for business logic
#[allow(unused_imports)]
use crate::domain::product::model::{
    Category, CreateCategory, CreateProduct, CreateUnit, Product, Unit, UpdateProduct,
};
use crate::domain::product::repository::{
    BoxCategoryRepository, BoxProductRepository, BoxUnitRepository,
};
use crate::error::ApiError;

/// Product service
#[derive(Clone)]
pub struct ProductService {
    product_repo: BoxProductRepository,
    category_repo: BoxCategoryRepository,
    unit_repo: BoxUnitRepository,
}

impl ProductService {
    pub fn new(
        product_repo: BoxProductRepository,
        category_repo: BoxCategoryRepository,
        unit_repo: BoxUnitRepository,
    ) -> Self {
        Self {
            product_repo,
            category_repo,
            unit_repo,
        }
    }

    // Product operations
    pub async fn create_product(&self, create: CreateProduct) -> Result<Product, ApiError> {
        create
            .validate()
            .map_err(|e| ApiError::Validation(e.join(", ")))?;

        // Check if code exists for this tenant
        if self
            .product_repo
            .find_by_code(create.tenant_id, &create.code)
            .await?
            .is_some()
        {
            return Err(ApiError::Conflict(format!(
                "Product code '{}' already exists",
                create.code
            )));
        }

        self.product_repo.create(create).await
    }

    pub async fn get_product(&self, id: i64) -> Result<Product, ApiError> {
        self.product_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Product {} not found", id)))
    }

    pub async fn get_products_by_tenant(&self, tenant_id: i64) -> Result<Vec<Product>, ApiError> {
        self.product_repo.find_by_tenant(tenant_id).await
    }

    pub async fn search_products(
        &self,
        tenant_id: i64,
        query: &str,
    ) -> Result<Vec<Product>, ApiError> {
        self.product_repo.search(tenant_id, query).await
    }

    pub async fn update_product(
        &self,
        id: i64,
        update: UpdateProduct,
    ) -> Result<Product, ApiError> {
        self.product_repo.update(id, update).await
    }

    pub async fn delete_product(&self, id: i64) -> Result<(), ApiError> {
        self.product_repo.delete(id).await
    }

    // Category operations
    pub async fn create_category(&self, create: CreateCategory) -> Result<Category, ApiError> {
        create
            .validate()
            .map_err(|e| ApiError::Validation(e.join(", ")))?;
        self.category_repo.create(create).await
    }

    pub async fn get_category(&self, id: i64) -> Result<Category, ApiError> {
        self.category_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Category {} not found", id)))
    }

    pub async fn get_categories_by_tenant(
        &self,
        tenant_id: i64,
    ) -> Result<Vec<Category>, ApiError> {
        self.category_repo.find_by_tenant(tenant_id).await
    }

    pub async fn delete_category(&self, id: i64) -> Result<(), ApiError> {
        self.category_repo.delete(id).await
    }

    // Unit operations
    pub async fn create_unit(&self, create: CreateUnit) -> Result<Unit, ApiError> {
        create
            .validate()
            .map_err(|e| ApiError::Validation(e.join(", ")))?;
        self.unit_repo.create(create).await
    }

    pub async fn get_unit(&self, id: i64) -> Result<Unit, ApiError> {
        self.unit_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Unit {} not found", id)))
    }

    pub async fn get_units_by_tenant(&self, tenant_id: i64) -> Result<Vec<Unit>, ApiError> {
        self.unit_repo.find_by_tenant(tenant_id).await
    }

    pub async fn delete_unit(&self, id: i64) -> Result<(), ApiError> {
        self.unit_repo.delete(id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::product::repository::{
        InMemoryCategoryRepository, InMemoryProductRepository, InMemoryUnitRepository,
    };
    use std::sync::Arc;

    fn create_service() -> ProductService {
        let product_repo = Arc::new(InMemoryProductRepository::new()) as BoxProductRepository;
        let category_repo = Arc::new(InMemoryCategoryRepository::new()) as BoxCategoryRepository;
        let unit_repo = Arc::new(InMemoryUnitRepository::new()) as BoxUnitRepository;
        ProductService::new(product_repo, category_repo, unit_repo)
    }

    #[tokio::test]
    async fn test_create_product_success() {
        let service = create_service();

        let create = CreateProduct {
            tenant_id: 1,
            code: "P001".to_string(),
            name: "Test Product".to_string(),
            description: None,
            category_id: None,
            unit_id: None,
            barcode: None,
            purchase_price: 100.0,
            sale_price: 150.0,
            tax_rate: 18.0,
        };

        let result = service.create_product(create).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().name, "Test Product");
    }

    #[tokio::test]
    async fn test_create_product_duplicate_code() {
        let service = create_service();

        let create = CreateProduct {
            tenant_id: 1,
            code: "P001".to_string(),
            name: "Test Product".to_string(),
            description: None,
            category_id: None,
            unit_id: None,
            barcode: None,
            purchase_price: 100.0,
            sale_price: 150.0,
            tax_rate: 18.0,
        };

        service.create_product(create.clone()).await.unwrap();
        let result = service.create_product(create).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ApiError::Conflict(_)));
    }

    #[tokio::test]
    async fn test_search_products() {
        let service = create_service();

        let create = CreateProduct {
            tenant_id: 1,
            code: "P001".to_string(),
            name: "Test Product".to_string(),
            description: None,
            category_id: None,
            unit_id: None,
            barcode: None,
            purchase_price: 100.0,
            sale_price: 150.0,
            tax_rate: 18.0,
        };

        service.create_product(create).await.unwrap();

        let result = service.search_products(1, "test").await.unwrap();
        assert!(!result.is_empty());

        let result = service.search_products(1, "nonexistent").await.unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_create_category() {
        let service = create_service();

        let create = CreateCategory {
            tenant_id: 1,
            name: "Electronics".to_string(),
            parent_id: None,
        };

        let result = service.create_category(create).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().name, "Electronics");
    }

    #[tokio::test]
    async fn test_create_unit() {
        let service = create_service();

        let create = CreateUnit {
            tenant_id: 1,
            code: "PCS".to_string(),
            name: "Piece".to_string(),
            is_integer: true,
        };

        let result = service.create_unit(create).await;
        assert!(result.is_ok());
    }
}
