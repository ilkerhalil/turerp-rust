//! Sales domain module

pub mod model;
pub mod postgres_repository;
pub mod repository;
pub mod service;

// Re-exports
pub use model::{
    CreateQuotation, CreateQuotationLine, CreateSalesOrder, CreateSalesOrderLine, Quotation,
    QuotationLine, QuotationResponse, QuotationStatus, SalesOrder, SalesOrderLine,
    SalesOrderResponse, SalesOrderStatus,
};
pub use postgres_repository::{
    PostgresQuotationLineRepository, PostgresQuotationRepository, PostgresSalesOrderLineRepository,
    PostgresSalesOrderRepository,
};
pub use repository::{
    BoxQuotationLineRepository, BoxQuotationRepository, BoxSalesOrderLineRepository,
    BoxSalesOrderRepository, InMemoryQuotationLineRepository, InMemoryQuotationRepository,
    InMemorySalesOrderLineRepository, InMemorySalesOrderRepository, QuotationLineRepository,
    QuotationRepository, SalesOrderLineRepository, SalesOrderRepository,
};
pub use service::SalesService;
