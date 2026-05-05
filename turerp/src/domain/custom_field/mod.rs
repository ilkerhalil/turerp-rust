//! Custom field definitions for dynamic module attributes

pub mod model;
pub mod repository;
pub mod service;

#[cfg(feature = "postgres")]
pub mod postgres_repository;

pub use model::{
    CreateCustomFieldDefinition, CustomFieldDefinition, CustomFieldDefinitionResponse,
    CustomFieldModule, CustomFieldType, CustomFieldValues, UpdateCustomFieldDefinition,
};
pub use repository::{
    BoxCustomFieldRepository, CustomFieldRepository, InMemoryCustomFieldRepository,
};
pub use service::CustomFieldService;
