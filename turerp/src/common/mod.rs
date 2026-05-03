//! Common utilities and types shared across modules

pub mod pagination;

pub use pagination::{PaginatedResult, PaginationParams};

use serde::Serialize;
use utoipa::ToSchema;

/// Simple localized success message payload.
#[derive(Serialize, ToSchema)]
pub struct MessageResponse {
    pub message: String,
}
