//! Purchase domain module

pub mod model;
pub mod repository;
pub mod service;

// Re-exports
pub use model::{
    CreateGoodsReceipt, CreateGoodsReceiptLine, CreatePurchaseOrder, CreatePurchaseOrderLine,
    GoodsReceipt, GoodsReceiptLine, GoodsReceiptResponse, GoodsReceiptStatus, PurchaseOrder,
    PurchaseOrderLine, PurchaseOrderResponse, PurchaseOrderStatus,
};
pub use repository::{
    BoxGoodsReceiptLineRepository, BoxGoodsReceiptRepository, BoxPurchaseOrderLineRepository,
    BoxPurchaseOrderRepository, GoodsReceiptLineRepository, GoodsReceiptRepository,
    InMemoryGoodsReceiptLineRepository, InMemoryGoodsReceiptRepository,
    InMemoryPurchaseOrderLineRepository, InMemoryPurchaseOrderRepository,
    PurchaseOrderLineRepository, PurchaseOrderRepository,
};
pub use service::PurchaseService;
