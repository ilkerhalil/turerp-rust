//! API Version 1

pub mod accounting;
pub mod assets;
pub mod audit;
pub mod auth;
pub mod cari;
pub mod crm;
pub mod feature_flags;
pub mod hr;
pub mod invoice;
pub mod manufacturing;
pub mod product_variants;
pub mod project;
pub mod purchase_requests;
pub mod sales;
pub mod settings;
pub mod stock;
pub mod tenant;
pub mod users;

// Explicit re-exports to avoid ambiguity
pub use accounting::configure as accounting_configure;
pub use assets::configure as assets_configure;
pub use audit::configure as audit_configure;
pub use auth::configure as auth_configure;
pub use cari::configure as cari_configure;
pub use crm::configure as crm_configure;
pub use feature_flags::configure as feature_flags_configure;
pub use hr::configure as hr_configure;
pub use invoice::configure as invoice_configure;
pub use manufacturing::configure as manufacturing_configure;
pub use product_variants::configure as product_variants_configure;
pub use project::configure as project_configure;
pub use purchase_requests::configure as purchase_requests_configure;
pub use sales::configure as sales_configure;
pub use settings::configure as settings_configure;
pub use stock::configure as stock_configure;
pub use tenant::configure as tenant_configure;
pub use users::configure as users_configure;
