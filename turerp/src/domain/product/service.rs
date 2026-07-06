//! Product service for business logic
use std::sync::Arc;

use crate::cache::{cache_get, cache_key, cache_set, CacheService};
use crate::common::pagination::PaginatedResult;
use crate::domain::company::service::ensure_company_owned;
use crate::domain::company::BoxCompanyRepository;
use crate::domain::product::model::{
    Category, CreateCategory, CreateProduct, CreateProductVariant, CreateUnit, Product,
    ProductVariantResponse, Unit, UpdateCategory, UpdateProduct, UpdateProductVariant, UpdateUnit,
};
use crate::domain::product::repository::{
    BoxCategoryRepository, BoxProductRepository, BoxProductVariantRepository, BoxUnitRepository,
};
use crate::error::ApiError;

/// TTL for product catalog cache entries (seconds)
const CATALOG_TTL: u64 = 120;

/// Product service
#[derive(Clone)]
pub struct ProductService {
    product_repo: BoxProductRepository,
    category_repo: BoxCategoryRepository,
    unit_repo: BoxUnitRepository,
    company_repo: BoxCompanyRepository,
    variant_repo: Option<BoxProductVariantRepository>,
    cache: Option<Arc<dyn CacheService>>,
}

impl ProductService {
    pub fn new(
        product_repo: BoxProductRepository,
        category_repo: BoxCategoryRepository,
        unit_repo: BoxUnitRepository,
        company_repo: BoxCompanyRepository,
    ) -> Self {
        Self {
            product_repo,
            category_repo,
            unit_repo,
            company_repo,
            variant_repo: None,
            cache: None,
        }
    }

    /// Create service with variant repository
    pub fn with_variants(
        product_repo: BoxProductRepository,
        category_repo: BoxCategoryRepository,
        unit_repo: BoxUnitRepository,
        variant_repo: BoxProductVariantRepository,
        company_repo: BoxCompanyRepository,
    ) -> Self {
        Self {
            product_repo,
            category_repo,
            unit_repo,
            company_repo,
            variant_repo: Some(variant_repo),
            cache: None,
        }
    }

    /// Attach a cache service for catalog caching
    pub fn with_cache(mut self, cache: Arc<dyn CacheService>) -> Self {
        self.cache = Some(cache);
        self
    }

    /// Invalidate product catalog cache for a tenant
    async fn invalidate_product_cache(&self, tenant_id: i64) {
        if let Some(ref cache) = self.cache {
            let pattern = cache_key(tenant_id, "products", "*");
            cache.delete_pattern(&pattern).await.ok();
        }
    }

    /// Invalidate category catalog cache for a tenant
    async fn invalidate_category_cache(&self, tenant_id: i64) {
        if let Some(ref cache) = self.cache {
            let pattern = cache_key(tenant_id, "categories", "*");
            cache.delete_pattern(&pattern).await.ok();
        }
    }

    /// Invalidate unit catalog cache for a tenant
    async fn invalidate_unit_cache(&self, tenant_id: i64) {
        if let Some(ref cache) = self.cache {
            let pattern = cache_key(tenant_id, "units", "*");
            cache.delete_pattern(&pattern).await.ok();
        }
    }

    // Product operations
    #[tracing::instrument(skip(self))]
    pub async fn create_product(&self, create: CreateProduct) -> Result<Product, ApiError> {
        create
            .validate()
            .map_err(|e| ApiError::Validation(e.join(", ")))?;

        // Parent-ownership precheck: body-controlled company_id must belong to the
        // caller's tenant (the legacy `1` sentinel is skipped for backward compat).
        ensure_company_owned(&self.company_repo, create.company_id, create.tenant_id).await?;

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

        let product = self.product_repo.create(create).await?;
        self.invalidate_product_cache(product.tenant_id).await;
        Ok(product)
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_product(&self, id: i64, tenant_id: i64) -> Result<Product, ApiError> {
        let ck = cache_key(tenant_id, "products", &id.to_string());

        if let Some(ref cache) = self.cache {
            if let Some(cached) = cache_get::<Product>(&**cache, &ck).await? {
                return Ok(cached);
            }
        }

        let product = self
            .product_repo
            .find_by_id(id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Product {} not found", id)))?;

        if let Some(ref cache) = self.cache {
            cache_set(&**cache, &ck, &product, Some(CATALOG_TTL))
                .await
                .ok();
        }

        Ok(product)
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_products_batch(
        &self,
        ids: &[i64],
        tenant_id: i64,
    ) -> Result<Vec<Product>, ApiError> {
        self.product_repo.find_by_ids(ids, tenant_id).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_products_by_tenant(&self, tenant_id: i64) -> Result<Vec<Product>, ApiError> {
        let ck = cache_key(tenant_id, "products", "all");

        if let Some(ref cache) = self.cache {
            if let Some(cached) = cache_get::<Vec<Product>>(&**cache, &ck).await? {
                return Ok(cached);
            }
        }

        let products = self.product_repo.find_by_tenant(tenant_id).await?;

        if let Some(ref cache) = self.cache {
            cache_set(&**cache, &ck, &products, Some(CATALOG_TTL))
                .await
                .ok();
        }

        Ok(products)
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_products_paginated(
        &self,
        tenant_id: i64,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<Product>, ApiError> {
        let params = crate::common::pagination::PaginationParams { page, per_page };
        params.validate().map_err(ApiError::Validation)?;

        let ck = cache_key(
            tenant_id,
            "products",
            &format!("list:{}:{}", page, per_page),
        );

        if let Some(ref cache) = self.cache {
            if let Some(cached) = cache_get::<PaginatedResult<Product>>(&**cache, &ck).await? {
                return Ok(cached);
            }
        }

        let result = self
            .product_repo
            .find_by_tenant_paginated(tenant_id, page, per_page)
            .await?;

        if let Some(ref cache) = self.cache {
            cache_set(&**cache, &ck, &result, Some(CATALOG_TTL))
                .await
                .ok();
        }

        Ok(result)
    }

    #[tracing::instrument(skip(self))]
    pub async fn search_products(
        &self,
        tenant_id: i64,
        query: &str,
    ) -> Result<Vec<Product>, ApiError> {
        // Search is not cached — always fresh results
        self.product_repo.search(tenant_id, query).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn update_product(
        &self,
        id: i64,
        tenant_id: i64,
        update: UpdateProduct,
    ) -> Result<Product, ApiError> {
        let product = self.product_repo.update(id, tenant_id, update).await?;
        self.invalidate_product_cache(tenant_id).await;
        Ok(product)
    }

    #[tracing::instrument(skip(self))]
    pub async fn delete_product(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        self.product_repo.delete(id, tenant_id).await?;
        self.invalidate_product_cache(tenant_id).await;
        Ok(())
    }

    /// Soft delete a product (sets deleted_at)
    #[tracing::instrument(skip(self))]
    pub async fn soft_delete_product(
        &self,
        id: i64,
        tenant_id: i64,
        deleted_by: i64,
    ) -> Result<(), ApiError> {
        self.product_repo
            .soft_delete(id, tenant_id, deleted_by)
            .await?;
        self.invalidate_product_cache(tenant_id).await;
        Ok(())
    }

    /// Restore a soft-deleted product
    #[tracing::instrument(skip(self))]
    pub async fn restore_product(&self, id: i64, tenant_id: i64) -> Result<Product, ApiError> {
        let product = self.product_repo.restore(id, tenant_id).await?;
        self.invalidate_product_cache(tenant_id).await;
        Ok(product)
    }

    /// List soft-deleted products
    #[tracing::instrument(skip(self))]
    pub async fn list_deleted_products(&self, tenant_id: i64) -> Result<Vec<Product>, ApiError> {
        self.product_repo.find_deleted(tenant_id).await
    }

    /// Permanently delete a product (hard delete)
    #[tracing::instrument(skip(self))]
    pub async fn destroy_product(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        self.product_repo.destroy(id, tenant_id).await?;
        self.invalidate_product_cache(tenant_id).await;
        Ok(())
    }

    // Category operations
    #[tracing::instrument(skip(self))]
    pub async fn create_category(&self, create: CreateCategory) -> Result<Category, ApiError> {
        create
            .validate()
            .map_err(|e| ApiError::Validation(e.join(", ")))?;
        // Parent-ownership precheck: body-controlled company_id must belong to the
        // caller's tenant (the legacy `1` sentinel is skipped for backward compat).
        ensure_company_owned(&self.company_repo, create.company_id, create.tenant_id).await?;
        let category = self.category_repo.create(create).await?;
        self.invalidate_category_cache(category.tenant_id).await;
        Ok(category)
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_category(&self, id: i64, tenant_id: i64) -> Result<Category, ApiError> {
        let ck = cache_key(tenant_id, "categories", &id.to_string());

        if let Some(ref cache) = self.cache {
            if let Some(cached) = cache_get::<Category>(&**cache, &ck).await? {
                return Ok(cached);
            }
        }

        let category = self
            .category_repo
            .find_by_id(id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Category {} not found", id)))?;

        if let Some(ref cache) = self.cache {
            cache_set(&**cache, &ck, &category, Some(CATALOG_TTL))
                .await
                .ok();
        }

        Ok(category)
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_categories_by_tenant(
        &self,
        tenant_id: i64,
    ) -> Result<Vec<Category>, ApiError> {
        let ck = cache_key(tenant_id, "categories", "all");

        if let Some(ref cache) = self.cache {
            if let Some(cached) = cache_get::<Vec<Category>>(&**cache, &ck).await? {
                return Ok(cached);
            }
        }

        let categories = self.category_repo.find_by_tenant(tenant_id).await?;

        if let Some(ref cache) = self.cache {
            cache_set(&**cache, &ck, &categories, Some(CATALOG_TTL))
                .await
                .ok();
        }

        Ok(categories)
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_categories_paginated(
        &self,
        tenant_id: i64,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<Category>, ApiError> {
        let params = crate::common::pagination::PaginationParams { page, per_page };
        params.validate().map_err(ApiError::Validation)?;

        let ck = cache_key(
            tenant_id,
            "categories",
            &format!("list:{}:{}", page, per_page),
        );

        if let Some(ref cache) = self.cache {
            if let Some(cached) = cache_get::<PaginatedResult<Category>>(&**cache, &ck).await? {
                return Ok(cached);
            }
        }

        let result = self
            .category_repo
            .find_by_tenant_paginated(tenant_id, page, per_page)
            .await?;

        if let Some(ref cache) = self.cache {
            cache_set(&**cache, &ck, &result, Some(CATALOG_TTL))
                .await
                .ok();
        }

        Ok(result)
    }

    #[tracing::instrument(skip(self))]
    pub async fn update_category(
        &self,
        id: i64,
        tenant_id: i64,
        update: UpdateCategory,
    ) -> Result<Category, ApiError> {
        let category = self.category_repo.update(id, tenant_id, update).await?;
        self.invalidate_category_cache(tenant_id).await;
        Ok(category)
    }

    #[tracing::instrument(skip(self))]
    pub async fn delete_category(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        self.category_repo.delete(id, tenant_id).await?;
        self.invalidate_category_cache(tenant_id).await;
        Ok(())
    }

    /// Soft delete a category (sets deleted_at)
    #[tracing::instrument(skip(self))]
    pub async fn soft_delete_category(
        &self,
        id: i64,
        tenant_id: i64,
        deleted_by: i64,
    ) -> Result<(), ApiError> {
        self.category_repo
            .soft_delete(id, tenant_id, deleted_by)
            .await?;
        self.invalidate_category_cache(tenant_id).await;
        Ok(())
    }

    /// Restore a soft-deleted category
    #[tracing::instrument(skip(self))]
    pub async fn restore_category(&self, id: i64, tenant_id: i64) -> Result<Category, ApiError> {
        let category = self.category_repo.restore(id, tenant_id).await?;
        self.invalidate_category_cache(tenant_id).await;
        Ok(category)
    }

    /// List soft-deleted categories
    #[tracing::instrument(skip(self))]
    pub async fn list_deleted_categories(&self, tenant_id: i64) -> Result<Vec<Category>, ApiError> {
        self.category_repo.find_deleted(tenant_id).await
    }

    /// Permanently delete a category (hard delete)
    #[tracing::instrument(skip(self))]
    pub async fn destroy_category(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        self.category_repo.destroy(id, tenant_id).await?;
        self.invalidate_category_cache(tenant_id).await;
        Ok(())
    }

    // Unit operations
    #[tracing::instrument(skip(self))]
    pub async fn create_unit(&self, create: CreateUnit) -> Result<Unit, ApiError> {
        create
            .validate()
            .map_err(|e| ApiError::Validation(e.join(", ")))?;
        // Parent-ownership precheck: body-controlled company_id must belong to the
        // caller's tenant (the legacy `1` sentinel is skipped for backward compat).
        ensure_company_owned(&self.company_repo, create.company_id, create.tenant_id).await?;
        let unit = self.unit_repo.create(create).await?;
        self.invalidate_unit_cache(unit.tenant_id).await;
        Ok(unit)
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_unit(&self, id: i64, tenant_id: i64) -> Result<Unit, ApiError> {
        let ck = cache_key(tenant_id, "units", &id.to_string());

        if let Some(ref cache) = self.cache {
            if let Some(cached) = cache_get::<Unit>(&**cache, &ck).await? {
                return Ok(cached);
            }
        }

        let unit = self
            .unit_repo
            .find_by_id(id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Unit {} not found", id)))?;

        if let Some(ref cache) = self.cache {
            cache_set(&**cache, &ck, &unit, Some(CATALOG_TTL))
                .await
                .ok();
        }

        Ok(unit)
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_units_by_tenant(&self, tenant_id: i64) -> Result<Vec<Unit>, ApiError> {
        let ck = cache_key(tenant_id, "units", "all");

        if let Some(ref cache) = self.cache {
            if let Some(cached) = cache_get::<Vec<Unit>>(&**cache, &ck).await? {
                return Ok(cached);
            }
        }

        let units = self.unit_repo.find_by_tenant(tenant_id).await?;

        if let Some(ref cache) = self.cache {
            cache_set(&**cache, &ck, &units, Some(CATALOG_TTL))
                .await
                .ok();
        }

        Ok(units)
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_units_paginated(
        &self,
        tenant_id: i64,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<Unit>, ApiError> {
        let params = crate::common::pagination::PaginationParams { page, per_page };
        params.validate().map_err(ApiError::Validation)?;

        let ck = cache_key(tenant_id, "units", &format!("list:{}:{}", page, per_page));

        if let Some(ref cache) = self.cache {
            if let Some(cached) = cache_get::<PaginatedResult<Unit>>(&**cache, &ck).await? {
                return Ok(cached);
            }
        }

        let result = self
            .unit_repo
            .find_by_tenant_paginated(tenant_id, page, per_page)
            .await?;

        if let Some(ref cache) = self.cache {
            cache_set(&**cache, &ck, &result, Some(CATALOG_TTL))
                .await
                .ok();
        }

        Ok(result)
    }

    #[tracing::instrument(skip(self))]
    pub async fn update_unit(
        &self,
        id: i64,
        tenant_id: i64,
        update: UpdateUnit,
    ) -> Result<Unit, ApiError> {
        let unit = self.unit_repo.update(id, tenant_id, update).await?;
        self.invalidate_unit_cache(tenant_id).await;
        Ok(unit)
    }

    #[tracing::instrument(skip(self))]
    pub async fn delete_unit(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        self.unit_repo.delete(id, tenant_id).await?;
        self.invalidate_unit_cache(tenant_id).await;
        Ok(())
    }

    /// Soft delete a unit (sets deleted_at)
    #[tracing::instrument(skip(self))]
    pub async fn soft_delete_unit(
        &self,
        id: i64,
        tenant_id: i64,
        deleted_by: i64,
    ) -> Result<(), ApiError> {
        self.unit_repo
            .soft_delete(id, tenant_id, deleted_by)
            .await?;
        self.invalidate_unit_cache(tenant_id).await;
        Ok(())
    }

    /// Restore a soft-deleted unit
    #[tracing::instrument(skip(self))]
    pub async fn restore_unit(&self, id: i64, tenant_id: i64) -> Result<Unit, ApiError> {
        let unit = self.unit_repo.restore(id, tenant_id).await?;
        self.invalidate_unit_cache(tenant_id).await;
        Ok(unit)
    }

    /// List soft-deleted units
    #[tracing::instrument(skip(self))]
    pub async fn list_deleted_units(&self, tenant_id: i64) -> Result<Vec<Unit>, ApiError> {
        self.unit_repo.find_deleted(tenant_id).await
    }

    /// Permanently delete a unit (hard delete)
    #[tracing::instrument(skip(self))]
    pub async fn destroy_unit(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        self.unit_repo.destroy(id, tenant_id).await?;
        self.invalidate_unit_cache(tenant_id).await;
        Ok(())
    }

    // Product variant operations
    #[tracing::instrument(skip(self))]
    pub async fn create_variant(
        &self,
        create: CreateProductVariant,
        tenant_id: i64,
    ) -> Result<ProductVariantResponse, ApiError> {
        let variant_repo = self
            .variant_repo
            .as_ref()
            .ok_or_else(|| ApiError::Internal("Variant repository not configured".to_string()))?;

        create
            .validate()
            .map_err(|e| ApiError::Validation(e.join(", ")))?;

        // Verify product exists (with tenant isolation)
        self.product_repo
            .find_by_id(create.product_id, tenant_id)
            .await?
            .ok_or_else(|| {
                ApiError::NotFound(format!("Product {} not found", create.product_id))
            })?;

        let variant = variant_repo.create(create).await?;
        Ok(variant.into())
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_variants_by_product(
        &self,
        product_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<ProductVariantResponse>, ApiError> {
        let variant_repo = self
            .variant_repo
            .as_ref()
            .ok_or_else(|| ApiError::Internal("Variant repository not configured".to_string()))?;

        // Verify product exists (with tenant isolation)
        self.product_repo
            .find_by_id(product_id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Product {} not found", product_id)))?;

        let variants = variant_repo.find_by_product(product_id, tenant_id).await?;
        Ok(variants.into_iter().map(|v| v.into()).collect())
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_variant(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<ProductVariantResponse, ApiError> {
        let variant_repo = self
            .variant_repo
            .as_ref()
            .ok_or_else(|| ApiError::Internal("Variant repository not configured".to_string()))?;

        let variant = variant_repo
            .find_by_id(id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Product variant {} not found", id)))?;
        Ok(variant.into())
    }

    #[tracing::instrument(skip(self))]
    pub async fn update_variant(
        &self,
        id: i64,
        tenant_id: i64,
        update: UpdateProductVariant,
    ) -> Result<ProductVariantResponse, ApiError> {
        let variant_repo = self
            .variant_repo
            .as_ref()
            .ok_or_else(|| ApiError::Internal("Variant repository not configured".to_string()))?;

        let variant = variant_repo.update(id, tenant_id, update).await?;
        Ok(variant.into())
    }

    #[tracing::instrument(skip(self))]
    pub async fn delete_variant(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let variant_repo = self
            .variant_repo
            .as_ref()
            .ok_or_else(|| ApiError::Internal("Variant repository not configured".to_string()))?;

        variant_repo.delete(id, tenant_id).await
    }

    /// Soft delete a product variant (sets deleted_at)
    #[tracing::instrument(skip(self))]
    pub async fn soft_delete_variant(
        &self,
        id: i64,
        tenant_id: i64,
        deleted_by: i64,
    ) -> Result<(), ApiError> {
        let variant_repo = self
            .variant_repo
            .as_ref()
            .ok_or_else(|| ApiError::Internal("Variant repository not configured".to_string()))?;

        variant_repo.soft_delete(id, tenant_id, deleted_by).await
    }

    /// Restore a soft-deleted product variant
    #[tracing::instrument(skip(self))]
    pub async fn restore_variant(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<ProductVariantResponse, ApiError> {
        let variant_repo = self
            .variant_repo
            .as_ref()
            .ok_or_else(|| ApiError::Internal("Variant repository not configured".to_string()))?;

        let variant = variant_repo.restore(id, tenant_id).await?;
        Ok(variant.into())
    }

    /// List soft-deleted product variants for a product
    #[tracing::instrument(skip(self))]
    pub async fn list_deleted_variants(
        &self,
        product_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<ProductVariantResponse>, ApiError> {
        let variant_repo = self
            .variant_repo
            .as_ref()
            .ok_or_else(|| ApiError::Internal("Variant repository not configured".to_string()))?;

        let variants = variant_repo.find_deleted(product_id, tenant_id).await?;
        Ok(variants.into_iter().map(|v| v.into()).collect())
    }

    /// Permanently delete a product variant (hard delete)
    #[tracing::instrument(skip(self))]
    pub async fn destroy_variant(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let variant_repo = self
            .variant_repo
            .as_ref()
            .ok_or_else(|| ApiError::Internal("Variant repository not configured".to_string()))?;

        variant_repo.destroy(id, tenant_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::NoopCacheService;
    use crate::domain::company::repository::InMemoryCompanyRepository;
    use crate::domain::company::service::LEGACY_COMPANY_ID;
    use crate::domain::company::{CompanyRepository, CreateCompany};
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
        let company_repo = Arc::new(InMemoryCompanyRepository::new()) as BoxCompanyRepository;
        ProductService::new(product_repo, category_repo, unit_repo, company_repo)
    }

    fn create_service_with_cache() -> ProductService {
        let product_repo = Arc::new(InMemoryProductRepository::new()) as BoxProductRepository;
        let category_repo = Arc::new(InMemoryCategoryRepository::new()) as BoxCategoryRepository;
        let unit_repo = Arc::new(InMemoryUnitRepository::new()) as BoxUnitRepository;
        let company_repo = Arc::new(InMemoryCompanyRepository::new()) as BoxCompanyRepository;
        ProductService::new(product_repo, category_repo, unit_repo, company_repo)
            .with_cache(Arc::new(NoopCacheService))
    }

    fn create_service_with_variants() -> ProductService {
        let product_repo = Arc::new(InMemoryProductRepository::new()) as BoxProductRepository;
        let category_repo = Arc::new(InMemoryCategoryRepository::new()) as BoxCategoryRepository;
        let unit_repo = Arc::new(InMemoryUnitRepository::new()) as BoxUnitRepository;
        let variant_repo = Arc::new(InMemoryProductVariantRepository::new(product_repo.clone()))
            as BoxProductVariantRepository;
        let company_repo = Arc::new(InMemoryCompanyRepository::new()) as BoxCompanyRepository;
        ProductService::with_variants(
            product_repo,
            category_repo,
            unit_repo,
            variant_repo,
            company_repo,
        )
    }

    /// Seed a company for `tenant_id` and return its id. Mirrors the cari test
    /// helper: the InMemory company repo auto-id starts at 1 (= LEGACY_COMPANY_ID
    /// sentinel, which the precheck skips), so to obtain a non-sentinel id for a
    /// real foreign-tenant company we first seed a tenant-1 company to consume
    /// id=1, then seed the target tenant's company (id=2).
    async fn seed_company(repo: &BoxCompanyRepository, tenant_id: i64, code: &str) -> i64 {
        repo.create(CreateCompany {
            code: code.to_string(),
            name: format!("Co-{}", code),
            tax_number: None,
            address: None,
            city: None,
            country: None,
            currency: "TRY".to_string(),
            tenant_id,
        })
        .await
        .map(|c| c.id)
        .expect("seed company")
    }

    #[tokio::test]
    async fn test_create_product_success() {
        let service = create_service();

        let create = CreateProduct {
            tenant_id: 1,
            company_id: 1,
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
            company_id: 1,
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
            company_id: 1,
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
            company_id: 1,
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
            company_id: 1,
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
            company_id: 1,
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

        let result = service.create_variant(variant_create, 1).await;
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
            company_id: 1,
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

        service.create_variant(variant_create1, 1).await.unwrap();
        service.create_variant(variant_create2, 1).await.unwrap();

        let variants = service
            .get_variants_by_product(product.id, 1)
            .await
            .unwrap();
        assert_eq!(variants.len(), 2);
    }

    #[tokio::test]
    async fn test_update_variant() {
        let service = create_service_with_variants();

        // Create product first
        let product_create = CreateProduct {
            tenant_id: 1,
            company_id: 1,
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
        let variant = service.create_variant(variant_create, 1).await.unwrap();

        // Update variant
        let update = UpdateProductVariant {
            name: Some("Updated Name".to_string()),
            sku: Some("NEW-SKU".to_string()),
            barcode: None,
            price_modifier: Some(dec!(15.0)),
            is_active: None,
        };

        let updated = service.update_variant(variant.id, 1, update).await.unwrap();
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
            company_id: 1,
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
        let variant = service.create_variant(variant_create, 1).await.unwrap();

        // Delete variant
        service.delete_variant(variant.id, 1).await.unwrap();

        // Verify deletion
        let result = service.get_variant(variant.id, 1).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_soft_delete_and_restore_product() {
        let service = create_service();

        let create = CreateProduct {
            tenant_id: 1,
            company_id: 1,
            code: "P-SOFT".to_string(),
            name: "Soft Delete Test".to_string(),
            description: None,
            category_id: None,
            unit_id: None,
            barcode: None,
            purchase_price: dec!(100.0),
            sale_price: dec!(150.0),
            tax_rate: dec!(18.0),
        };

        let product = service.create_product(create).await.unwrap();
        let id = product.id;

        // Soft delete
        service.soft_delete_product(id, 1, 1).await.unwrap();

        // Should not be found via normal get
        assert!(service.get_product(id, 1).await.is_err());

        // Should appear in deleted list
        let deleted = service.list_deleted_products(1).await.unwrap();
        assert!(deleted.iter().any(|p| p.id == id));

        // Restore
        let restored = service.restore_product(id, 1).await.unwrap();
        assert_eq!(restored.id, id);

        // Should be found again
        assert!(service.get_product(id, 1).await.is_ok());
    }

    #[tokio::test]
    async fn test_soft_delete_and_restore_category() {
        let service = create_service();

        let create = CreateCategory {
            tenant_id: 1,
            company_id: 1,
            name: "Test Category".to_string(),
            parent_id: None,
        };

        let category = service.create_category(create).await.unwrap();
        let id = category.id;

        // Soft delete
        service.soft_delete_category(id, 1, 1).await.unwrap();

        // Should not be found via normal get
        assert!(service.get_category(id, 1).await.is_err());

        // Restore
        let restored = service.restore_category(id, 1).await.unwrap();
        assert_eq!(restored.id, id);
    }

    #[tokio::test]
    async fn test_soft_delete_and_restore_unit() {
        let service = create_service();

        let create = CreateUnit {
            tenant_id: 1,
            company_id: 1,
            code: "KG".to_string(),
            name: "Kilogram".to_string(),
            is_integer: false,
        };

        let unit = service.create_unit(create).await.unwrap();
        let id = unit.id;

        // Soft delete
        service.soft_delete_unit(id, 1, 1).await.unwrap();

        // Should not be found via normal get
        assert!(service.get_unit(id, 1).await.is_err());

        // Restore
        let restored = service.restore_unit(id, 1).await.unwrap();
        assert_eq!(restored.id, id);
    }

    #[tokio::test]
    async fn test_catalog_cache_with_noop() {
        let service = create_service_with_cache();

        let create = CreateProduct {
            tenant_id: 1,
            company_id: 1,
            code: "P-CACHE".to_string(),
            name: "Cache Test".to_string(),
            description: None,
            category_id: None,
            unit_id: None,
            barcode: None,
            purchase_price: dec!(100.0),
            sale_price: dec!(150.0),
            tax_rate: dec!(18.0),
        };

        service.create_product(create).await.unwrap();

        // With NoopCache, this should still work (fallback to DB)
        let products = service.get_products_by_tenant(1).await.unwrap();
        assert!(!products.is_empty());

        let paginated = service.get_products_paginated(1, 1, 10).await.unwrap();
        assert!(!paginated.items.is_empty());
    }

    /// Regression test for commit `fix(security): tenant_id for product_variants`.
    /// `product_variants` has no `tenant_id` column of its own; tenant isolation
    /// must be enforced via the parent product. A variant created under
    /// tenant-1's product must not be readable / mutable / deletable by tenant-2.
    #[tokio::test]
    async fn test_variant_tenant_isolation() {
        let service = create_service_with_variants();

        // Product owned by tenant 1
        let p1 = service
            .create_product(CreateProduct {
                tenant_id: 1,
                company_id: 1,
                code: "P-ISO-1".to_string(),
                name: "Tenant1 Product".to_string(),
                description: None,
                category_id: None,
                unit_id: None,
                barcode: None,
                purchase_price: dec!(10.0),
                sale_price: dec!(20.0),
                tax_rate: dec!(18.0),
            })
            .await
            .unwrap();

        // Variant under tenant 1's product
        let v = service
            .create_variant(
                CreateProductVariant {
                    product_id: p1.id,
                    name: "Tenant1 Variant".to_string(),
                    sku: Some("T1-V".to_string()),
                    barcode: None,
                    price_modifier: dec!(0.0),
                },
                1,
            )
            .await
            .unwrap();

        // Tenant 2 cannot read it
        let from_tenant_2 = service.get_variant(v.id, 2).await;
        assert!(
            matches!(from_tenant_2, Err(ApiError::NotFound(_))),
            "tenant-2 should NOT be able to read tenant-1's variant, got {:?}",
            from_tenant_2
        );

        // Tenant 2 cannot list it via the parent product lookup
        let list_result = service.get_variants_by_product(p1.id, 2).await;
        assert!(
            matches!(list_result, Err(ApiError::NotFound(_))),
            "tenant-2 must NOT see tenant-1's parent product, got {:?}",
            list_result
        );

        // Tenant 2 cannot update it
        let upd = UpdateProductVariant {
            name: Some("Hacked".to_string()),
            sku: None,
            barcode: None,
            price_modifier: None,
            is_active: None,
        };
        let update_result = service.update_variant(v.id, 2, upd).await;
        assert!(
            matches!(update_result, Err(ApiError::NotFound(_))),
            "tenant-2 should NOT be able to update tenant-1's variant, got {:?}",
            update_result
        );

        // Tenant 2 cannot soft-delete it
        let del_result = service.soft_delete_variant(v.id, 2, 999).await;
        assert!(
            matches!(del_result, Err(ApiError::NotFound(_))),
            "tenant-2 should NOT be able to soft-delete tenant-1's variant, got {:?}",
            del_result
        );

        // Tenant 2 cannot hard-delete it
        let destroy_result = service.destroy_variant(v.id, 2).await;
        assert!(
            matches!(destroy_result, Err(ApiError::NotFound(_))),
            "tenant-2 should NOT be able to destroy tenant-1's variant, got {:?}",
            destroy_result
        );

        // Tenant 1 CAN still read it (sanity check — we didn't break tenant-1)
        let from_tenant_1 = service.get_variant(v.id, 1).await.unwrap();
        assert_eq!(from_tenant_1.id, v.id);
        assert_eq!(from_tenant_1.name, "Tenant1 Variant");
    }

    // ---- company_id parent-ownership precheck (cross-tenant IDOR) ----

    #[tokio::test]
    async fn test_create_product_rejects_foreign_company() {
        let service = create_service();
        let company_repo = service.company_repo.clone();

        // Consume the id=1 sentinel with a tenant-1 company, so the tenant-2
        // foreign company gets a non-sentinel id (the precheck only skips id=1).
        seed_company(&company_repo, 1, "T1").await;
        let foreign_company_id = seed_company(&company_repo, 2, "T2").await;
        assert_ne!(foreign_company_id, LEGACY_COMPANY_ID);

        // A tenant-1 caller stamps a tenant-2 company id → must be rejected.
        let create = CreateProduct {
            tenant_id: 1,
            company_id: foreign_company_id,
            code: "P-FOR".to_string(),
            name: "Foreign Co Product".to_string(),
            description: None,
            category_id: None,
            unit_id: None,
            barcode: None,
            purchase_price: dec!(100.0),
            sale_price: dec!(150.0),
            tax_rate: dec!(18.0),
        };
        let result = service.create_product(create).await;
        assert!(
            matches!(result, Err(ApiError::NotFound(_))),
            "tenant-1 must NOT create a product under a tenant-2 company, got {:?}",
            result
        );
    }

    #[tokio::test]
    async fn test_create_category_rejects_foreign_company() {
        let service = create_service();
        let company_repo = service.company_repo.clone();

        seed_company(&company_repo, 1, "T1").await;
        let foreign_company_id = seed_company(&company_repo, 2, "T2").await;
        assert_ne!(foreign_company_id, LEGACY_COMPANY_ID);

        let create = CreateCategory {
            tenant_id: 1,
            company_id: foreign_company_id,
            name: "Foreign Co Category".to_string(),
            parent_id: None,
        };
        let result = service.create_category(create).await;
        assert!(
            matches!(result, Err(ApiError::NotFound(_))),
            "tenant-1 must NOT create a category under a tenant-2 company, got {:?}",
            result
        );
    }

    #[tokio::test]
    async fn test_create_unit_rejects_foreign_company() {
        let service = create_service();
        let company_repo = service.company_repo.clone();

        seed_company(&company_repo, 1, "T1").await;
        let foreign_company_id = seed_company(&company_repo, 2, "T2").await;
        assert_ne!(foreign_company_id, LEGACY_COMPANY_ID);

        let create = CreateUnit {
            tenant_id: 1,
            company_id: foreign_company_id,
            code: "U-FOR".to_string(),
            name: "Foreign Co Unit".to_string(),
            is_integer: false,
        };
        let result = service.create_unit(create).await;
        assert!(
            matches!(result, Err(ApiError::NotFound(_))),
            "tenant-1 must NOT create a unit under a tenant-2 company, got {:?}",
            result
        );
    }
}
