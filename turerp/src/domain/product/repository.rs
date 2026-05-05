//! Product repository

use async_trait::async_trait;
use parking_lot::Mutex;
use std::sync::Arc;

use crate::common::pagination::PaginatedResult;
use crate::common::SoftDeletable;
use crate::domain::product::model::{
    Category, CreateCategory, CreateProduct, CreateProductVariant, CreateUnit, Product,
    ProductVariant, Unit, UpdateProduct, UpdateProductVariant,
};
use crate::error::ApiError;

/// Repository trait for Product operations
#[async_trait]
pub trait ProductRepository: Send + Sync {
    async fn create(&self, product: CreateProduct) -> Result<Product, ApiError>;
    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<Product>, ApiError>;
    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<Product>, ApiError>;
    async fn find_by_tenant_paginated(
        &self,
        tenant_id: i64,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<Product>, ApiError>;
    async fn find_by_code(&self, tenant_id: i64, code: &str) -> Result<Option<Product>, ApiError>;
    async fn search(&self, tenant_id: i64, query: &str) -> Result<Vec<Product>, ApiError>;
    async fn update(
        &self,
        id: i64,
        tenant_id: i64,
        product: UpdateProduct,
    ) -> Result<Product, ApiError>;
    async fn delete(&self, id: i64, tenant_id: i64) -> Result<(), ApiError>;
    /// Soft delete a product
    async fn soft_delete(&self, id: i64, tenant_id: i64, deleted_by: i64) -> Result<(), ApiError>;
    /// Restore a soft-deleted product
    async fn restore(&self, id: i64, tenant_id: i64) -> Result<Product, ApiError>;
    /// Find soft-deleted products
    async fn find_deleted(&self, tenant_id: i64) -> Result<Vec<Product>, ApiError>;
    /// Hard delete a product (permanent destruction)
    async fn destroy(&self, id: i64, tenant_id: i64) -> Result<(), ApiError>;
}

/// Repository trait for Category operations
#[async_trait]
pub trait CategoryRepository: Send + Sync {
    async fn create(&self, category: CreateCategory) -> Result<Category, ApiError>;
    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<Category>, ApiError>;
    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<Category>, ApiError>;
    async fn find_by_tenant_paginated(
        &self,
        tenant_id: i64,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<Category>, ApiError>;
    async fn delete(&self, id: i64, tenant_id: i64) -> Result<(), ApiError>;
    /// Soft delete a category
    async fn soft_delete(&self, id: i64, tenant_id: i64, deleted_by: i64) -> Result<(), ApiError>;
    /// Restore a soft-deleted category
    async fn restore(&self, id: i64, tenant_id: i64) -> Result<Category, ApiError>;
    /// Find soft-deleted categories
    async fn find_deleted(&self, tenant_id: i64) -> Result<Vec<Category>, ApiError>;
    /// Hard delete a category (permanent destruction)
    async fn destroy(&self, id: i64, tenant_id: i64) -> Result<(), ApiError>;
}

/// Repository trait for Unit operations
#[async_trait]
pub trait UnitRepository: Send + Sync {
    async fn create(&self, unit: CreateUnit) -> Result<Unit, ApiError>;
    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<Unit>, ApiError>;
    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<Unit>, ApiError>;
    async fn delete(&self, id: i64, tenant_id: i64) -> Result<(), ApiError>;
    /// Soft delete a unit
    async fn soft_delete(&self, id: i64, tenant_id: i64, deleted_by: i64) -> Result<(), ApiError>;
    /// Restore a soft-deleted unit
    async fn restore(&self, id: i64, tenant_id: i64) -> Result<Unit, ApiError>;
    /// Find soft-deleted units
    async fn find_deleted(&self, tenant_id: i64) -> Result<Vec<Unit>, ApiError>;
    /// Hard delete a unit (permanent destruction)
    async fn destroy(&self, id: i64, tenant_id: i64) -> Result<(), ApiError>;
}

/// Repository trait for ProductVariant operations.
/// Note: ProductVariant does not have a tenant_id field; it is a child entity of Product.
/// Tenant isolation should be enforced by looking up the parent Product first.
#[async_trait]
pub trait ProductVariantRepository: Send + Sync {
    async fn create(&self, variant: CreateProductVariant) -> Result<ProductVariant, ApiError>;
    async fn find_by_product(&self, product_id: i64) -> Result<Vec<ProductVariant>, ApiError>;
    async fn find_by_id(&self, id: i64) -> Result<Option<ProductVariant>, ApiError>;
    async fn update(
        &self,
        id: i64,
        variant: UpdateProductVariant,
    ) -> Result<ProductVariant, ApiError>;
    async fn delete(&self, id: i64) -> Result<(), ApiError>;
    /// Soft delete a product variant
    async fn soft_delete(&self, id: i64, deleted_by: i64) -> Result<(), ApiError>;
    /// Restore a soft-deleted product variant
    async fn restore(&self, id: i64) -> Result<ProductVariant, ApiError>;
    /// Find soft-deleted product variants
    async fn find_deleted(&self, product_id: i64) -> Result<Vec<ProductVariant>, ApiError>;
    /// Hard delete a product variant (permanent destruction)
    async fn destroy(&self, id: i64) -> Result<(), ApiError>;
}

/// Type aliases
pub type BoxProductRepository = Arc<dyn ProductRepository>;
pub type BoxCategoryRepository = Arc<dyn CategoryRepository>;
pub type BoxUnitRepository = Arc<dyn UnitRepository>;
pub type BoxProductVariantRepository = Arc<dyn ProductVariantRepository>;

/// Inner state for InMemoryProductRepository
struct InMemoryProductInner {
    products: std::collections::HashMap<i64, Product>,
    next_id: i64,
    tenant_products: std::collections::HashMap<i64, Vec<i64>>,
}

/// In-memory product repository for testing
pub struct InMemoryProductRepository {
    inner: Mutex<InMemoryProductInner>,
}

impl InMemoryProductRepository {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(InMemoryProductInner {
                products: std::collections::HashMap::new(),
                next_id: 1,
                tenant_products: std::collections::HashMap::new(),
            }),
        }
    }
}

impl Default for InMemoryProductRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ProductRepository for InMemoryProductRepository {
    async fn create(&self, create: CreateProduct) -> Result<Product, ApiError> {
        let mut inner = self.inner.lock();
        let id = inner.next_id;
        inner.next_id += 1;

        let now = chrono::Utc::now();
        let product = Product {
            id,
            tenant_id: create.tenant_id,
            code: create.code,
            name: create.name,
            description: create.description,
            category_id: create.category_id,
            unit_id: create.unit_id,
            barcode: create.barcode,
            purchase_price: create.purchase_price,
            sale_price: create.sale_price,
            tax_rate: create.tax_rate,
            is_active: true,
            created_at: now,
            updated_at: now,
            deleted_at: None,
            deleted_by: None,
        };

        inner.products.insert(id, product.clone());
        inner
            .tenant_products
            .entry(create.tenant_id)
            .or_default()
            .push(id);

        Ok(product)
    }

    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<Product>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .products
            .get(&id)
            .filter(|p| p.tenant_id == tenant_id && !p.is_deleted())
            .cloned())
    }

    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<Product>, ApiError> {
        let inner = self.inner.lock();
        let ids = inner
            .tenant_products
            .get(&tenant_id)
            .cloned()
            .unwrap_or_default();
        Ok(ids
            .iter()
            .filter_map(|id| inner.products.get(id).cloned())
            .filter(|p| !p.is_deleted())
            .collect())
    }

    async fn find_by_tenant_paginated(
        &self,
        tenant_id: i64,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<Product>, ApiError> {
        let inner = self.inner.lock();
        let all: Vec<_> = inner
            .products
            .values()
            .filter(|p| p.tenant_id == tenant_id && !p.is_deleted())
            .cloned()
            .collect();
        let total = all.len() as u64;
        let items: Vec<_> = all
            .into_iter()
            .skip(((page.saturating_sub(1)) * per_page) as usize)
            .take(per_page as usize)
            .collect();
        Ok(PaginatedResult::new(items, page, per_page, total))
    }

    async fn find_by_code(&self, tenant_id: i64, code: &str) -> Result<Option<Product>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .products
            .values()
            .find(|p| p.tenant_id == tenant_id && p.code == code && !p.is_deleted())
            .cloned())
    }

    async fn search(&self, tenant_id: i64, query: &str) -> Result<Vec<Product>, ApiError> {
        let query_lower = query.to_lowercase();
        let inner = self.inner.lock();

        Ok(inner
            .products
            .values()
            .filter(|p| {
                p.tenant_id == tenant_id
                    && !p.is_deleted()
                    && (p.name.to_lowercase().contains(&query_lower)
                        || p.code.to_lowercase().contains(&query_lower))
            })
            .cloned()
            .collect())
    }

    async fn update(
        &self,
        id: i64,
        tenant_id: i64,
        update: UpdateProduct,
    ) -> Result<Product, ApiError> {
        let mut inner = self.inner.lock();

        let product = inner
            .products
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound(format!("Product {} not found", id)))?;

        if product.tenant_id != tenant_id {
            return Err(ApiError::NotFound(format!("Product {} not found", id)));
        }

        if let Some(code) = update.code {
            product.code = code;
        }
        if let Some(name) = update.name {
            product.name = name;
        }
        if let Some(description) = update.description {
            product.description = Some(description);
        }
        if let Some(category_id) = update.category_id {
            product.category_id = Some(category_id);
        }
        if let Some(unit_id) = update.unit_id {
            product.unit_id = Some(unit_id);
        }
        if let Some(barcode) = update.barcode {
            product.barcode = Some(barcode);
        }
        if let Some(purchase_price) = update.purchase_price {
            product.purchase_price = purchase_price;
        }
        if let Some(sale_price) = update.sale_price {
            product.sale_price = sale_price;
        }
        if let Some(tax_rate) = update.tax_rate {
            product.tax_rate = tax_rate;
        }
        if let Some(is_active) = update.is_active {
            product.is_active = is_active;
        }
        product.updated_at = chrono::Utc::now();

        Ok(product.clone())
    }

    async fn delete(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();

        let product = inner
            .products
            .get(&id)
            .ok_or_else(|| ApiError::NotFound(format!("Product {} not found", id)))?;

        if product.tenant_id != tenant_id {
            return Err(ApiError::NotFound(format!("Product {} not found", id)));
        }

        inner.products.remove(&id);

        if let Some(ids) = inner.tenant_products.get_mut(&tenant_id) {
            ids.retain(|x| *x != id);
        }

        Ok(())
    }

    async fn soft_delete(&self, id: i64, tenant_id: i64, deleted_by: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();

        let product = inner
            .products
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound(format!("Product {} not found", id)))?;

        if product.tenant_id != tenant_id {
            return Err(ApiError::NotFound(format!("Product {} not found", id)));
        }

        product.mark_deleted(deleted_by);
        Ok(())
    }

    async fn restore(&self, id: i64, tenant_id: i64) -> Result<Product, ApiError> {
        let mut inner = self.inner.lock();

        let product = inner
            .products
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound(format!("Product {} not found", id)))?;

        if product.tenant_id != tenant_id {
            return Err(ApiError::NotFound(format!("Product {} not found", id)));
        }

        product.restore();
        Ok(product.clone())
    }

    async fn find_deleted(&self, tenant_id: i64) -> Result<Vec<Product>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .products
            .values()
            .filter(|p| p.tenant_id == tenant_id && p.is_deleted())
            .cloned()
            .collect())
    }

    async fn destroy(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();

        let product = inner
            .products
            .get(&id)
            .ok_or_else(|| ApiError::NotFound(format!("Product {} not found", id)))?;

        if product.tenant_id != tenant_id {
            return Err(ApiError::NotFound(format!("Product {} not found", id)));
        }

        inner.products.remove(&id);

        if let Some(ids) = inner.tenant_products.get_mut(&tenant_id) {
            ids.retain(|x| *x != id);
        }

        Ok(())
    }
}

/// Inner state for InMemoryCategoryRepository
struct InMemoryCategoryInner {
    categories: std::collections::HashMap<i64, Category>,
    next_id: i64,
}

/// In-memory category repository
pub struct InMemoryCategoryRepository {
    inner: Mutex<InMemoryCategoryInner>,
}

impl InMemoryCategoryRepository {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(InMemoryCategoryInner {
                categories: std::collections::HashMap::new(),
                next_id: 1,
            }),
        }
    }
}

impl Default for InMemoryCategoryRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CategoryRepository for InMemoryCategoryRepository {
    async fn create(&self, create: CreateCategory) -> Result<Category, ApiError> {
        let mut inner = self.inner.lock();
        let id = inner.next_id;
        inner.next_id += 1;

        let category = Category {
            id,
            tenant_id: create.tenant_id,
            name: create.name,
            parent_id: create.parent_id,
            created_at: chrono::Utc::now(),
            deleted_at: None,
            deleted_by: None,
        };

        inner.categories.insert(id, category.clone());
        Ok(category)
    }

    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<Category>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .categories
            .get(&id)
            .filter(|c| c.tenant_id == tenant_id && !c.is_deleted())
            .cloned())
    }

    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<Category>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .categories
            .values()
            .filter(|c| c.tenant_id == tenant_id && !c.is_deleted())
            .cloned()
            .collect())
    }

    async fn find_by_tenant_paginated(
        &self,
        tenant_id: i64,
        page: u32,
        per_page: u32,
    ) -> Result<PaginatedResult<Category>, ApiError> {
        let inner = self.inner.lock();
        let all: Vec<_> = inner
            .categories
            .values()
            .filter(|c| c.tenant_id == tenant_id && !c.is_deleted())
            .cloned()
            .collect();
        let total = all.len() as u64;
        let items: Vec<_> = all
            .into_iter()
            .skip(((page.saturating_sub(1)) * per_page) as usize)
            .take(per_page as usize)
            .collect();
        Ok(PaginatedResult::new(items, page, per_page, total))
    }

    async fn delete(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();
        let category = inner
            .categories
            .get(&id)
            .ok_or_else(|| ApiError::NotFound(format!("Category {} not found", id)))?;

        if category.tenant_id != tenant_id {
            return Err(ApiError::NotFound(format!("Category {} not found", id)));
        }

        inner.categories.remove(&id);
        Ok(())
    }

    async fn soft_delete(&self, id: i64, tenant_id: i64, deleted_by: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();

        let category = inner
            .categories
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound(format!("Category {} not found", id)))?;

        if category.tenant_id != tenant_id {
            return Err(ApiError::NotFound(format!("Category {} not found", id)));
        }

        category.mark_deleted(deleted_by);
        Ok(())
    }

    async fn restore(&self, id: i64, tenant_id: i64) -> Result<Category, ApiError> {
        let mut inner = self.inner.lock();

        let category = inner
            .categories
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound(format!("Category {} not found", id)))?;

        if category.tenant_id != tenant_id {
            return Err(ApiError::NotFound(format!("Category {} not found", id)));
        }

        category.restore();
        Ok(category.clone())
    }

    async fn find_deleted(&self, tenant_id: i64) -> Result<Vec<Category>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .categories
            .values()
            .filter(|c| c.tenant_id == tenant_id && c.is_deleted())
            .cloned()
            .collect())
    }

    async fn destroy(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();
        let category = inner
            .categories
            .get(&id)
            .ok_or_else(|| ApiError::NotFound(format!("Category {} not found", id)))?;

        if category.tenant_id != tenant_id {
            return Err(ApiError::NotFound(format!("Category {} not found", id)));
        }

        inner.categories.remove(&id);
        Ok(())
    }
}

/// Inner state for InMemoryUnitRepository
struct InMemoryUnitInner {
    units: std::collections::HashMap<i64, Unit>,
    next_id: i64,
}

/// In-memory unit repository
pub struct InMemoryUnitRepository {
    inner: Mutex<InMemoryUnitInner>,
}

impl InMemoryUnitRepository {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(InMemoryUnitInner {
                units: std::collections::HashMap::new(),
                next_id: 1,
            }),
        }
    }

    pub fn with_defaults(tenant_id: i64) -> Self {
        let repo = Self::new();
        let defaults = vec![
            (1, tenant_id, "PCS", "Piece", true),
            (2, tenant_id, "KG", "Kilogram", false),
            (3, tenant_id, "BOX", "Box", true),
            (4, tenant_id, "MT", "Meter", false),
            (5, tenant_id, "L", "Liter", false),
        ];

        let mut inner = repo.inner.lock();
        for (id, tid, code, name, is_int) in defaults {
            inner.units.insert(
                id,
                Unit {
                    id,
                    tenant_id: tid,
                    code: code.to_string(),
                    name: name.to_string(),
                    is_integer: is_int,
                    created_at: chrono::Utc::now(),
                    deleted_at: None,
                    deleted_by: None,
                },
            );
        }
        inner.next_id = 6;
        drop(inner);
        repo
    }
}

impl Default for InMemoryUnitRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl UnitRepository for InMemoryUnitRepository {
    async fn create(&self, create: CreateUnit) -> Result<Unit, ApiError> {
        let mut inner = self.inner.lock();
        let id = inner.next_id;
        inner.next_id += 1;

        let unit = Unit {
            id,
            tenant_id: create.tenant_id,
            code: create.code,
            name: create.name,
            is_integer: create.is_integer,
            created_at: chrono::Utc::now(),
            deleted_at: None,
            deleted_by: None,
        };

        inner.units.insert(id, unit.clone());
        Ok(unit)
    }

    async fn find_by_id(&self, id: i64, tenant_id: i64) -> Result<Option<Unit>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .units
            .get(&id)
            .filter(|u| u.tenant_id == tenant_id && !u.is_deleted())
            .cloned())
    }

    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<Unit>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .units
            .values()
            .filter(|u| u.tenant_id == tenant_id && !u.is_deleted())
            .cloned()
            .collect())
    }

    async fn delete(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();
        let unit = inner
            .units
            .get(&id)
            .ok_or_else(|| ApiError::NotFound(format!("Unit {} not found", id)))?;

        if unit.tenant_id != tenant_id {
            return Err(ApiError::NotFound(format!("Unit {} not found", id)));
        }

        inner.units.remove(&id);
        Ok(())
    }

    async fn soft_delete(&self, id: i64, tenant_id: i64, deleted_by: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();

        let unit = inner
            .units
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound(format!("Unit {} not found", id)))?;

        if unit.tenant_id != tenant_id {
            return Err(ApiError::NotFound(format!("Unit {} not found", id)));
        }

        unit.mark_deleted(deleted_by);
        Ok(())
    }

    async fn restore(&self, id: i64, tenant_id: i64) -> Result<Unit, ApiError> {
        let mut inner = self.inner.lock();

        let unit = inner
            .units
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound(format!("Unit {} not found", id)))?;

        if unit.tenant_id != tenant_id {
            return Err(ApiError::NotFound(format!("Unit {} not found", id)));
        }

        unit.restore();
        Ok(unit.clone())
    }

    async fn find_deleted(&self, tenant_id: i64) -> Result<Vec<Unit>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .units
            .values()
            .filter(|u| u.tenant_id == tenant_id && u.is_deleted())
            .cloned()
            .collect())
    }

    async fn destroy(&self, id: i64, tenant_id: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();
        let unit = inner
            .units
            .get(&id)
            .ok_or_else(|| ApiError::NotFound(format!("Unit {} not found", id)))?;

        if unit.tenant_id != tenant_id {
            return Err(ApiError::NotFound(format!("Unit {} not found", id)));
        }

        inner.units.remove(&id);
        Ok(())
    }
}

/// Inner state for InMemoryProductVariantRepository
struct InMemoryProductVariantInner {
    variants: std::collections::HashMap<i64, ProductVariant>,
    next_id: i64,
    product_variants: std::collections::HashMap<i64, Vec<i64>>,
}

/// In-memory product variant repository
pub struct InMemoryProductVariantRepository {
    inner: Mutex<InMemoryProductVariantInner>,
}

impl InMemoryProductVariantRepository {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(InMemoryProductVariantInner {
                variants: std::collections::HashMap::new(),
                next_id: 1,
                product_variants: std::collections::HashMap::new(),
            }),
        }
    }
}

impl Default for InMemoryProductVariantRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ProductVariantRepository for InMemoryProductVariantRepository {
    async fn create(&self, create: CreateProductVariant) -> Result<ProductVariant, ApiError> {
        let mut inner = self.inner.lock();
        let id = inner.next_id;
        inner.next_id += 1;

        let variant = ProductVariant {
            id,
            product_id: create.product_id,
            name: create.name,
            sku: create.sku,
            barcode: create.barcode,
            price_modifier: create.price_modifier,
            is_active: true,
            created_at: chrono::Utc::now(),
            deleted_at: None,
            deleted_by: None,
        };

        inner.variants.insert(id, variant.clone());
        inner
            .product_variants
            .entry(create.product_id)
            .or_default()
            .push(id);

        Ok(variant)
    }

    async fn find_by_product(&self, product_id: i64) -> Result<Vec<ProductVariant>, ApiError> {
        let inner = self.inner.lock();
        let ids = inner
            .product_variants
            .get(&product_id)
            .cloned()
            .unwrap_or_default();
        Ok(ids
            .iter()
            .filter_map(|id| inner.variants.get(id).cloned())
            .filter(|v| !v.is_deleted())
            .collect())
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<ProductVariant>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner.variants.get(&id).filter(|v| !v.is_deleted()).cloned())
    }

    async fn update(
        &self,
        id: i64,
        update: UpdateProductVariant,
    ) -> Result<ProductVariant, ApiError> {
        let mut inner = self.inner.lock();

        let variant = inner
            .variants
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound(format!("Product variant {} not found", id)))?;

        if let Some(name) = update.name {
            variant.name = name;
        }
        if let Some(sku) = update.sku {
            variant.sku = Some(sku);
        }
        if let Some(barcode) = update.barcode {
            variant.barcode = Some(barcode);
        }
        if let Some(price_modifier) = update.price_modifier {
            variant.price_modifier = price_modifier;
        }
        if let Some(is_active) = update.is_active {
            variant.is_active = is_active;
        }

        Ok(variant.clone())
    }

    async fn delete(&self, id: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();

        if !inner.variants.contains_key(&id) {
            return Err(ApiError::NotFound(format!(
                "Product variant {} not found",
                id
            )));
        }

        let product_id = inner.variants.get(&id).map(|v| v.product_id);
        inner.variants.remove(&id);

        if let Some(pid) = product_id {
            if let Some(ids) = inner.product_variants.get_mut(&pid) {
                ids.retain(|x| *x != id);
            }
        }

        Ok(())
    }

    async fn soft_delete(&self, id: i64, deleted_by: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();

        let variant = inner
            .variants
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound(format!("Product variant {} not found", id)))?;

        variant.mark_deleted(deleted_by);
        Ok(())
    }

    async fn restore(&self, id: i64) -> Result<ProductVariant, ApiError> {
        let mut inner = self.inner.lock();

        let variant = inner
            .variants
            .get_mut(&id)
            .ok_or_else(|| ApiError::NotFound(format!("Product variant {} not found", id)))?;

        variant.restore();
        Ok(variant.clone())
    }

    async fn find_deleted(&self, product_id: i64) -> Result<Vec<ProductVariant>, ApiError> {
        let inner = self.inner.lock();
        let ids = inner
            .product_variants
            .get(&product_id)
            .cloned()
            .unwrap_or_default();
        Ok(ids
            .iter()
            .filter_map(|id| inner.variants.get(id).cloned())
            .filter(|v| v.is_deleted())
            .collect())
    }

    async fn destroy(&self, id: i64) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();

        let variant = inner
            .variants
            .get(&id)
            .ok_or_else(|| ApiError::NotFound(format!("Product variant {} not found", id)))?;

        let product_id = variant.product_id;
        inner.variants.remove(&id);

        if let Some(ids) = inner.product_variants.get_mut(&product_id) {
            ids.retain(|x| *x != id);
        }

        Ok(())
    }
}
