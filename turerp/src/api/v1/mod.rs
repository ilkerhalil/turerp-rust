//! API Version 1

pub mod auth;
pub mod feature_flags;
pub mod product_variants;
pub mod purchase_requests;
pub mod users;

// Explicit re-exports to avoid ambiguity
pub use auth::configure as auth_configure;
pub use feature_flags::configure as feature_flags_configure;
pub use product_variants::configure as product_variants_configure;
pub use purchase_requests::configure as purchase_requests_configure;
pub use users::configure as users_configure;
