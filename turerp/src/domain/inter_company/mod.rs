//! Inter-company domain module for cross-company invoices and stock transfers.

pub mod model;
pub mod repository;
pub mod service;

pub use model::{
    CreateInterCompanyInvoice, CreateInterCompanyStockTransfer, InterCompanyInvoice,
    InterCompanyInvoiceLine, InterCompanyInvoiceResult, InterCompanyStockTransfer,
    InterCompanyStockTransferResult,
};
pub use repository::{
    BoxInterCompanyRepository, InMemoryInterCompanyRepository, InterCompanyRepository,
};
pub use service::InterCompanyService;
