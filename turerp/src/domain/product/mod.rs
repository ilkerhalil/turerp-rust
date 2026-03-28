//! Product domain module

pub mod model;
pub mod repository;
pub mod service;

// Re-exports
pub use model::{
    Category, CreateCategory, CreateProduct, CreateUnit, Product, ProductResponse, ProductVariant,
    Unit, UpdateProduct,
};
pub use repository::{
    BoxCategoryRepository, BoxProductRepository, BoxUnitRepository, CategoryRepository,
    InMemoryCategoryRepository, InMemoryProductRepository, InMemoryUnitRepository,
    ProductRepository, UnitRepository,
};
pub use service::ProductService;
