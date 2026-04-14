//! Product domain module

pub mod model;
#[cfg(feature = "postgres")]
pub mod postgres_repository;
pub mod repository;
pub mod service;

// Re-exports
pub use model::{
    Category, CreateCategory, CreateProduct, CreateProductVariant, CreateUnit, Product,
    ProductResponse, ProductVariant, ProductVariantResponse, Unit, UpdateProduct,
    UpdateProductVariant,
};
#[cfg(feature = "postgres")]
pub use postgres_repository::{
    PostgresCategoryRepository, PostgresProductRepository, PostgresProductVariantRepository,
    PostgresUnitRepository,
};
pub use repository::{
    BoxCategoryRepository, BoxProductRepository, BoxProductVariantRepository, BoxUnitRepository,
    CategoryRepository, InMemoryCategoryRepository, InMemoryProductRepository,
    InMemoryProductVariantRepository, InMemoryUnitRepository, ProductRepository,
    ProductVariantRepository, UnitRepository,
};
pub use service::ProductService;
