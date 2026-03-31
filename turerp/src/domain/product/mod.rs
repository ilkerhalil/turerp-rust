//! Product domain module

pub mod model;
pub mod repository;
pub mod service;

// Re-exports
pub use model::{
    Category, CreateCategory, CreateProduct, CreateProductVariant, CreateUnit, Product,
    ProductResponse, ProductVariant, ProductVariantResponse, Unit, UpdateProduct,
    UpdateProductVariant,
};
pub use repository::{
    BoxCategoryRepository, BoxProductRepository, BoxProductVariantRepository, BoxUnitRepository,
    CategoryRepository, InMemoryCategoryRepository, InMemoryProductRepository,
    InMemoryProductVariantRepository, InMemoryUnitRepository, ProductRepository,
    ProductVariantRepository, UnitRepository,
};
pub use service::ProductService;
