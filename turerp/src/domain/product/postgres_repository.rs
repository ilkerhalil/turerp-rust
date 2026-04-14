//! PostgreSQL product repository implementation

use async_trait::async_trait;
use rust_decimal::Decimal;
use sqlx::{FromRow, PgPool};
use std::sync::Arc;

use crate::domain::product::model::{
    Category, CreateCategory, CreateProduct, CreateProductVariant, CreateUnit, Product,
    ProductVariant, Unit, UpdateProduct, UpdateProductVariant,
};
use crate::domain::product::repository::{
    BoxCategoryRepository, BoxProductRepository, BoxProductVariantRepository, BoxUnitRepository,
    CategoryRepository, ProductRepository, ProductVariantRepository, UnitRepository,
};
use crate::error::ApiError;

/// Convert sqlx errors to ApiError with proper detection of error types
fn map_sqlx_error(e: sqlx::Error, entity: &str) -> ApiError {
    match e {
        sqlx::Error::RowNotFound => ApiError::NotFound(format!("{} not found", entity)),
        _ => {
            let msg = e.to_string();
            if msg.contains("duplicate key") || msg.contains("unique constraint") {
                ApiError::Conflict(format!("{} already exists", entity))
            } else {
                ApiError::Database(format!("Failed to operate on {}: {}", entity, e))
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Product
// ---------------------------------------------------------------------------

/// Database row representation for Product
#[derive(Debug, FromRow)]
struct ProductRow {
    id: i64,
    tenant_id: i64,
    code: String,
    name: String,
    description: Option<String>,
    category_id: Option<i64>,
    unit_id: Option<i64>,
    barcode: Option<String>,
    purchase_price: Decimal,
    sale_price: Decimal,
    tax_rate: Decimal,
    is_active: bool,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<ProductRow> for Product {
    fn from(row: ProductRow) -> Self {
        Self {
            id: row.id,
            tenant_id: row.tenant_id,
            code: row.code,
            name: row.name,
            description: row.description,
            category_id: row.category_id,
            unit_id: row.unit_id,
            barcode: row.barcode,
            purchase_price: row.purchase_price,
            sale_price: row.sale_price,
            tax_rate: row.tax_rate,
            is_active: row.is_active,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

/// PostgreSQL product repository
pub struct PostgresProductRepository {
    pool: Arc<PgPool>,
}

impl PostgresProductRepository {
    /// Create a new PostgreSQL product repository
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    /// Convert to boxed trait object
    pub fn into_boxed(self) -> BoxProductRepository {
        Arc::new(self) as BoxProductRepository
    }
}

#[async_trait]
impl ProductRepository for PostgresProductRepository {
    async fn create(&self, create: CreateProduct) -> Result<Product, ApiError> {
        let row: ProductRow = sqlx::query_as(
            r#"
            INSERT INTO products (tenant_id, code, name, description, category_id, unit_id,
                                  barcode, purchase_price, sale_price, tax_rate, is_active,
                                  created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, true, NOW(), NOW())
            RETURNING id, tenant_id, code, name, description, category_id, unit_id,
                      barcode, purchase_price, sale_price, tax_rate, is_active,
                      created_at, updated_at
            "#,
        )
        .bind(create.tenant_id)
        .bind(&create.code)
        .bind(&create.name)
        .bind(&create.description)
        .bind(create.category_id)
        .bind(create.unit_id)
        .bind(&create.barcode)
        .bind(create.purchase_price)
        .bind(create.sale_price)
        .bind(create.tax_rate)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Product"))?;

        Ok(row.into())
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<Product>, ApiError> {
        let result: Option<ProductRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, code, name, description, category_id, unit_id,
                   barcode, purchase_price, sale_price, tax_rate, is_active,
                   created_at, updated_at
            FROM products
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find product by id: {}", e)))?;

        Ok(result.map(|r| r.into()))
    }

    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<Product>, ApiError> {
        let rows: Vec<ProductRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, code, name, description, category_id, unit_id,
                   barcode, purchase_price, sale_price, tax_rate, is_active,
                   created_at, updated_at
            FROM products
            WHERE tenant_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find products by tenant: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn find_by_code(&self, tenant_id: i64, code: &str) -> Result<Option<Product>, ApiError> {
        let result: Option<ProductRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, code, name, description, category_id, unit_id,
                   barcode, purchase_price, sale_price, tax_rate, is_active,
                   created_at, updated_at
            FROM products
            WHERE tenant_id = $1 AND code = $2
            "#,
        )
        .bind(tenant_id)
        .bind(code)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find product by code: {}", e)))?;

        Ok(result.map(|r| r.into()))
    }

    async fn search(&self, tenant_id: i64, query: &str) -> Result<Vec<Product>, ApiError> {
        let pattern = format!("%{}%", query);

        let rows: Vec<ProductRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, code, name, description, category_id, unit_id,
                   barcode, purchase_price, sale_price, tax_rate, is_active,
                   created_at, updated_at
            FROM products
            WHERE tenant_id = $1
              AND (LOWER(code) LIKE LOWER($2) OR LOWER(name) LIKE LOWER($2))
            ORDER BY created_at DESC
            "#,
        )
        .bind(tenant_id)
        .bind(&pattern)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to search products: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn update(&self, id: i64, update: UpdateProduct) -> Result<Product, ApiError> {
        let row: ProductRow = sqlx::query_as(
            r#"
            UPDATE products
            SET
                code = COALESCE($1, code),
                name = COALESCE($2, name),
                description = COALESCE($3, description),
                category_id = COALESCE($4, category_id),
                unit_id = COALESCE($5, unit_id),
                barcode = COALESCE($6, barcode),
                purchase_price = COALESCE($7, purchase_price),
                sale_price = COALESCE($8, sale_price),
                tax_rate = COALESCE($9, tax_rate),
                is_active = COALESCE($10, is_active),
                updated_at = NOW()
            WHERE id = $11
            RETURNING id, tenant_id, code, name, description, category_id, unit_id,
                      barcode, purchase_price, sale_price, tax_rate, is_active,
                      created_at, updated_at
            "#,
        )
        .bind(&update.code)
        .bind(&update.name)
        .bind(&update.description)
        .bind(update.category_id)
        .bind(update.unit_id)
        .bind(&update.barcode)
        .bind(update.purchase_price)
        .bind(update.sale_price)
        .bind(update.tax_rate)
        .bind(update.is_active)
        .bind(id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Product"))?;

        Ok(row.into())
    }

    async fn delete(&self, id: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            DELETE FROM products
            WHERE id = $1
            "#,
        )
        .bind(id)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to delete product: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound("Product not found".to_string()));
        }

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Category
// ---------------------------------------------------------------------------

/// Database row representation for Category
#[derive(Debug, FromRow)]
struct CategoryRow {
    id: i64,
    tenant_id: i64,
    name: String,
    parent_id: Option<i64>,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl From<CategoryRow> for Category {
    fn from(row: CategoryRow) -> Self {
        Self {
            id: row.id,
            tenant_id: row.tenant_id,
            name: row.name,
            parent_id: row.parent_id,
            created_at: row.created_at,
        }
    }
}

/// PostgreSQL category repository
pub struct PostgresCategoryRepository {
    pool: Arc<PgPool>,
}

impl PostgresCategoryRepository {
    /// Create a new PostgreSQL category repository
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    /// Convert to boxed trait object
    pub fn into_boxed(self) -> BoxCategoryRepository {
        Arc::new(self) as BoxCategoryRepository
    }
}

#[async_trait]
impl CategoryRepository for PostgresCategoryRepository {
    async fn create(&self, create: CreateCategory) -> Result<Category, ApiError> {
        let row: CategoryRow = sqlx::query_as(
            r#"
            INSERT INTO categories (tenant_id, name, parent_id, created_at)
            VALUES ($1, $2, $3, NOW())
            RETURNING id, tenant_id, name, parent_id, created_at
            "#,
        )
        .bind(create.tenant_id)
        .bind(&create.name)
        .bind(create.parent_id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Category"))?;

        Ok(row.into())
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<Category>, ApiError> {
        let result: Option<CategoryRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, name, parent_id, created_at
            FROM categories
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find category by id: {}", e)))?;

        Ok(result.map(|r| r.into()))
    }

    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<Category>, ApiError> {
        let rows: Vec<CategoryRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, name, parent_id, created_at
            FROM categories
            WHERE tenant_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find categories by tenant: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn delete(&self, id: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            DELETE FROM categories
            WHERE id = $1
            "#,
        )
        .bind(id)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to delete category: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound("Category not found".to_string()));
        }

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Unit
// ---------------------------------------------------------------------------

/// Database row representation for Unit
#[derive(Debug, FromRow)]
struct UnitRow {
    id: i64,
    tenant_id: i64,
    code: String,
    name: String,
    is_integer: bool,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl From<UnitRow> for Unit {
    fn from(row: UnitRow) -> Self {
        Self {
            id: row.id,
            tenant_id: row.tenant_id,
            code: row.code,
            name: row.name,
            is_integer: row.is_integer,
            created_at: row.created_at,
        }
    }
}

/// PostgreSQL unit repository
pub struct PostgresUnitRepository {
    pool: Arc<PgPool>,
}

impl PostgresUnitRepository {
    /// Create a new PostgreSQL unit repository
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    /// Convert to boxed trait object
    pub fn into_boxed(self) -> BoxUnitRepository {
        Arc::new(self) as BoxUnitRepository
    }
}

#[async_trait]
impl UnitRepository for PostgresUnitRepository {
    async fn create(&self, create: CreateUnit) -> Result<Unit, ApiError> {
        let row: UnitRow = sqlx::query_as(
            r#"
            INSERT INTO units (tenant_id, code, name, is_integer, created_at)
            VALUES ($1, $2, $3, $4, NOW())
            RETURNING id, tenant_id, code, name, is_integer, created_at
            "#,
        )
        .bind(create.tenant_id)
        .bind(&create.code)
        .bind(&create.name)
        .bind(create.is_integer)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "Unit"))?;

        Ok(row.into())
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<Unit>, ApiError> {
        let result: Option<UnitRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, code, name, is_integer, created_at
            FROM units
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find unit by id: {}", e)))?;

        Ok(result.map(|r| r.into()))
    }

    async fn find_by_tenant(&self, tenant_id: i64) -> Result<Vec<Unit>, ApiError> {
        let rows: Vec<UnitRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, code, name, is_integer, created_at
            FROM units
            WHERE tenant_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(tenant_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find units by tenant: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn delete(&self, id: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            DELETE FROM units
            WHERE id = $1
            "#,
        )
        .bind(id)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to delete unit: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound("Unit not found".to_string()));
        }

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// ProductVariant
// ---------------------------------------------------------------------------

/// Database row representation for ProductVariant
#[derive(Debug, FromRow)]
struct ProductVariantRow {
    id: i64,
    product_id: i64,
    name: String,
    sku: Option<String>,
    barcode: Option<String>,
    price_modifier: Decimal,
    is_active: bool,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl From<ProductVariantRow> for ProductVariant {
    fn from(row: ProductVariantRow) -> Self {
        Self {
            id: row.id,
            product_id: row.product_id,
            name: row.name,
            sku: row.sku,
            barcode: row.barcode,
            price_modifier: row.price_modifier,
            is_active: row.is_active,
            created_at: row.created_at,
        }
    }
}

/// PostgreSQL product variant repository
pub struct PostgresProductVariantRepository {
    pool: Arc<PgPool>,
}

impl PostgresProductVariantRepository {
    /// Create a new PostgreSQL product variant repository
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    /// Convert to boxed trait object
    pub fn into_boxed(self) -> BoxProductVariantRepository {
        Arc::new(self) as BoxProductVariantRepository
    }
}

#[async_trait]
impl ProductVariantRepository for PostgresProductVariantRepository {
    async fn create(&self, create: CreateProductVariant) -> Result<ProductVariant, ApiError> {
        let row: ProductVariantRow = sqlx::query_as(
            r#"
            INSERT INTO product_variants (product_id, name, sku, barcode, price_modifier,
                                          is_active, created_at)
            VALUES ($1, $2, $3, $4, $5, true, NOW())
            RETURNING id, product_id, name, sku, barcode, price_modifier, is_active, created_at
            "#,
        )
        .bind(create.product_id)
        .bind(&create.name)
        .bind(&create.sku)
        .bind(&create.barcode)
        .bind(create.price_modifier)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "ProductVariant"))?;

        Ok(row.into())
    }

    async fn find_by_product(&self, product_id: i64) -> Result<Vec<ProductVariant>, ApiError> {
        let rows: Vec<ProductVariantRow> = sqlx::query_as(
            r#"
            SELECT id, product_id, name, sku, barcode, price_modifier, is_active, created_at
            FROM product_variants
            WHERE product_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(product_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| {
            ApiError::Database(format!("Failed to find product variants by product: {}", e))
        })?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn find_by_id(&self, id: i64) -> Result<Option<ProductVariant>, ApiError> {
        let result: Option<ProductVariantRow> = sqlx::query_as(
            r#"
            SELECT id, product_id, name, sku, barcode, price_modifier, is_active, created_at
            FROM product_variants
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find product variant by id: {}", e)))?;

        Ok(result.map(|r| r.into()))
    }

    async fn update(
        &self,
        id: i64,
        update: UpdateProductVariant,
    ) -> Result<ProductVariant, ApiError> {
        let row: ProductVariantRow = sqlx::query_as(
            r#"
            UPDATE product_variants
            SET
                name = COALESCE($1, name),
                sku = COALESCE($2, sku),
                barcode = COALESCE($3, barcode),
                price_modifier = COALESCE($4, price_modifier),
                is_active = COALESCE($5, is_active)
            WHERE id = $6
            RETURNING id, product_id, name, sku, barcode, price_modifier, is_active, created_at
            "#,
        )
        .bind(&update.name)
        .bind(&update.sku)
        .bind(&update.barcode)
        .bind(update.price_modifier)
        .bind(update.is_active)
        .bind(id)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "ProductVariant"))?;

        Ok(row.into())
    }

    async fn delete(&self, id: i64) -> Result<(), ApiError> {
        let result = sqlx::query(
            r#"
            DELETE FROM product_variants
            WHERE id = $1
            "#,
        )
        .bind(id)
        .execute(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to delete product variant: {}", e)))?;

        if result.rows_affected() == 0 {
            return Err(ApiError::NotFound("Product variant not found".to_string()));
        }

        Ok(())
    }
}
