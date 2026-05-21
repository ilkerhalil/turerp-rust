//! Inter-company domain module for cross-company invoices and stock transfers.

pub mod model;
pub mod service;

pub use model::{
    InterCompanyInvoiceLine, InterCompanyInvoiceResult, InterCompanyStockTransferResult,
};
pub use service::InterCompanyService;
