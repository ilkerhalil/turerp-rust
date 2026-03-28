//! Invoice domain module

pub mod model;
pub mod repository;
pub mod service;

// Re-exports
pub use model::{
    CreateInvoice, CreateInvoiceLine, CreatePayment, Invoice, InvoiceLine, InvoiceResponse,
    InvoiceStatus, InvoiceType, Payment,
};
pub use repository::{
    BoxInvoiceLineRepository, BoxInvoiceRepository, BoxPaymentRepository,
    InMemoryInvoiceLineRepository, InMemoryInvoiceRepository, InMemoryPaymentRepository,
    InvoiceLineRepository, InvoiceRepository, PaymentRepository,
};
pub use service::InvoiceService;
