//! Invoice domain module

pub mod model;
#[cfg(feature = "postgres")]
pub mod postgres_repository;
pub mod repository;
pub mod service;

// Re-exports
pub use model::{
    CreateInvoice, CreateInvoiceLine, CreatePayment, Invoice, InvoiceLine, InvoiceResponse,
    InvoiceStatus, InvoiceType, Payment,
};
#[cfg(feature = "postgres")]
pub use postgres_repository::{
    PostgresInvoiceLineRepository, PostgresInvoiceRepository, PostgresPaymentRepository,
};
pub use repository::{
    BoxInvoiceLineRepository, BoxInvoiceRepository, BoxPaymentRepository,
    InMemoryInvoiceLineRepository, InMemoryInvoiceRepository, InMemoryPaymentRepository,
    InvoiceLineRepository, InvoiceRepository, PaymentRepository,
};
pub use service::InvoiceService;
