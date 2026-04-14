//! Product service for business logic
use crate::common::pagination::PaginatedResult;
use crate::domain::product::model::{
    Category, CreateCategory, CreateProduct, CreateProductVariant, CreateUnit, Product,
    ProductVariantResponse, Unit, UpdateProduct, UpdateProductVariant,
};
use crate::domain::product::repository::{
    BoxCategoryRepository, BoxProductRepository, BoxProductVariantRepository, BoxUnitRepository,
};
use crate::error::ApiError;

/// Product service
#[derive(Clone)]
pub struct ProductService {
    product_repo: BoxProductRepository,
    category_repo: BoxCategoryRepository,
    unit_repo: BoxUnitRepository,
    variant_repo: Option<BoxProductVariantRepository>,
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
            variant_repo: None,
        }
    }

    /// Create service with variant repository
    pub fn with_variants(
        product_repo: BoxProductRepository,
        category_repo: BoxCategoryRepository,
        unit_repo: BoxUnitRepository,
        variant_repo: BoxProductVariantRepository,
    ) -> Self {
        Self {
            product_repo,
            category_repo,
            unit_repo,
            variant_repo: Some(variant_repo),
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

    pub async fn get_products_paginated(
        &self,
        tenant_id: i64,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<Product>, ApiError> {
        let params = crate::common::pagination::PaginationParams { page, per_page };
        params.validate().map_err(ApiError::Validation)?;
        self.product_repo
            .find_by_tenant_paginated(tenant_id, page, per_page)
            .await
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

    pub async fn get_categories_paginated(
        &self,
        tenant_id: i64,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<Category>, ApiError> {
        let params = crate::common::pagination::PaginationParams { page, per_page };
        params.validate().map_err(ApiError::Validation)?;
        self.category_repo
            .find_by_tenant_paginated(tenant_id, page, per_page)
            .await
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

    // Product variant operations
    pub async fn create_variant(
        &self,
        create: CreateProductVariant,
    ) -> Result<ProductVariantResponse, ApiError> {
        let variant_repo = self
            .variant_repo
            .as_ref()
            .ok_or_else(|| ApiError::Internal("Variant repository not configured".to_string()))?;

        create
            .validate()
            .map_err(|e| ApiError::Validation(e.join(", ")))?;

        // Verify product exists
        self.product_repo
            .find_by_id(create.product_id)
            .await?
            .ok_or_else(|| {
                ApiError::NotFound(format!("Product {} not found", create.product_id))
            })?;

        let variant = variant_repo.create(create).await?;
        Ok(variant.into())
    }

    pub async fn get_variants_by_product(
        &self,
        product_id: i64,
    ) -> Result<Vec<ProductVariantResponse>, ApiError> {
        let variant_repo = self
            .variant_repo
            .as_ref()
            .ok_or_else(|| ApiError::Internal("Variant repository not configured".to_string()))?;

        // Verify product exists
        self.product_repo
            .find_by_id(product_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Product {} not found", product_id)))?;

        let variants = variant_repo.find_by_product(product_id).await?;
        Ok(variants.into_iter().map(|v| v.into()).collect())
    }

    pub async fn get_variant(&self, id: i64) -> Result<ProductVariantResponse, ApiError> {
        let variant_repo = self
            .variant_repo
            .as_ref()
            .ok_or_else(|| ApiError::Internal("Variant repository not configured".to_string()))?;

        let variant = variant_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Product variant {} not found", id)))?;
        Ok(variant.into())
    }

    pub async fn update_variant(
        &self,
        id: i64,
        update: UpdateProductVariant,
    ) -> Result<ProductVariantResponse, ApiError> {
        let variant_repo = self
            .variant_repo
            .as_ref()
            .ok_or_else(|| ApiError::Internal("Variant repository not configured".to_string()))?;

        let variant = variant_repo.update(id, update).await?;
        Ok(variant.into())
    }

    pub async fn delete_variant(&self, id: i64) -> Result<(), ApiError> {
        let variant_repo = self
            .variant_repo
            .as_ref()
            .ok_or_else(|| ApiError::Internal("Variant repository not configured".to_string()))?;

        variant_repo.delete(id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::product::repository::{
        InMemoryCategoryRepository, InMemoryProductRepository, InMemoryProductVariantRepository,
        InMemoryUnitRepository,
    };
    use rust_decimal_macros::dec;
    use std::sync::Arc;

    fn create_service() -> ProductService {
        let product_repo = Arc::new(InMemoryProductRepository::new()) as BoxProductRepository;
        let category_repo = Arc::new(InMemoryCategoryRepository::new()) as BoxCategoryRepository;
        let unit_repo = Arc::new(InMemoryUnitRepository::new()) as BoxUnitRepository;
        ProductService::new(product_repo, category_repo, unit_repo)
    }

    fn create_service_with_variants() -> ProductService {
        let product_repo = Arc::new(InMemoryProductRepository::new()) as BoxProductRepository;
        let category_repo = Arc::new(InMemoryCategoryRepository::new()) as BoxCategoryRepository;
        let unit_repo = Arc::new(InMemoryUnitRepository::new()) as BoxUnitRepository;
        let variant_repo =
            Arc::new(InMemoryProductVariantRepository::new()) as BoxProductVariantRepository;
        ProductService::with_variants(product_repo, category_repo, unit_repo, variant_repo)
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
            purchase_price: dec!(100.0),
            sale_price: dec!(150.0),
            tax_rate: dec!(18.0),
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
            purchase_price: dec!(100.0),
            sale_price: dec!(150.0),
            tax_rate: dec!(18.0),
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
            purchase_price: dec!(100.0),
            sale_price: dec!(150.0),
            tax_rate: dec!(18.0),
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

    #[tokio::test]
    async fn test_create_variant() {
        let service = create_service_with_variants();

        // Create product first
        let product_create = CreateProduct {
            tenant_id: 1,
            code: "P001".to_string(),
            name: "Test Product".to_string(),
            description: None,
            category_id: None,
            unit_id: None,
            barcode: None,
            purchase_price: dec!(100.0),
            sale_price: dec!(150.0),
            tax_rate: dec!(18.0),
        };
        let product = service.create_product(product_create).await.unwrap();

        // Create variant
        let variant_create = CreateProductVariant {
            product_id: product.id,
            name: "Red Large".to_string(),
            sku: Some("P001-RED-L".to_string()),
            barcode: None,
            price_modifier: dec!(10.0),
        };

        let result = service.create_variant(variant_create).await;
        assert!(result.is_ok());
        let variant = result.unwrap();
        assert_eq!(variant.name, "Red Large");
        assert_eq!(variant.product_id, product.id);
    }

    #[tokio::test]
    async fn test_get_variants_by_product() {
        let service = create_service_with_variants();

        // Create product first
        let product_create = CreateProduct {
            tenant_id: 1,
            code: "P001".to_string(),
            name: "Test Product".to_string(),
            description: None,
            category_id: None,
            unit_id: None,
            barcode: None,
            purchase_price: dec!(100.0),
            sale_price: dec!(150.0),
            tax_rate: dec!(18.0),
        };
        let product = service.create_product(product_create).await.unwrap();

        // Create variants
        let variant_create1 = CreateProductVariant {
            product_id: product.id,
            name: "Red Large".to_string(),
            sku: Some("P001-RED-L".to_string()),
            barcode: None,
            price_modifier: dec!(10.0),
        };
        let variant_create2 = CreateProductVariant {
            product_id: product.id,
            name: "Blue Small".to_string(),
            sku: Some("P001-BLU-S".to_string()),
            barcode: None,
            price_modifier: dec!(-5.0),
        };

        service.create_variant(variant_create1).await.unwrap();
        service.create_variant(variant_create2).await.unwrap();

        let variants = service.get_variants_by_product(product.id).await.unwrap();
        assert_eq!(variants.len(), 2);
    }

    #[tokio::test]
    async fn test_update_variant() {
        let service = create_service_with_variants();

        // Create product first
        let product_create = CreateProduct {
            tenant_id: 1,
            code: "P001".to_string(),
            name: "Test Product".to_string(),
            description: None,
            category_id: None,
            unit_id: None,
            barcode: None,
            purchase_price: dec!(100.0),
            sale_price: dec!(150.0),
            tax_rate: dec!(18.0),
        };
        let product = service.create_product(product_create).await.unwrap();

        // Create variant
        let variant_create = CreateProductVariant {
            product_id: product.id,
            name: "Original Name".to_string(),
            sku: None,
            barcode: None,
            price_modifier: dec!(0.0),
        };
        let variant = service.create_variant(variant_create).await.unwrap();

        // Update variant
        let update = UpdateProductVariant {
            name: Some("Updated Name".to_string()),
            sku: Some("NEW-SKU".to_string()),
            barcode: None,
            price_modifier: Some(dec!(15.0)),
            is_active: None,
        };

        let updated = service.update_variant(variant.id, update).await.unwrap();
        assert_eq!(updated.name, "Updated Name");
        assert_eq!(updated.sku, Some("NEW-SKU".to_string()));
        assert_eq!(updated.price_modifier, dec!(15.0));
    }

    #[tokio::test]
    async fn test_delete_variant() {
        let service = create_service_with_variants();

        // Create product first
        let product_create = CreateProduct {
            tenant_id: 1,
            code: "P001".to_string(),
            name: "Test Product".to_string(),
            description: None,
            category_id: None,
            unit_id: None,
            barcode: None,
            purchase_price: dec!(100.0),
            sale_price: dec!(150.0),
            tax_rate: dec!(18.0),
        };
        let product = service.create_product(product_create).await.unwrap();

        // Create variant
        let variant_create = CreateProductVariant {
            product_id: product.id,
            name: "Test Variant".to_string(),
            sku: None,
            barcode: None,
            price_modifier: dec!(0.0),
        };
        let variant = service.create_variant(variant_create).await.unwrap();

        // Delete variant
        service.delete_variant(variant.id).await.unwrap();

        // Verify deletion
        let result = service.get_variant(variant.id).await;
        assert!(result.is_err());
    }
}
