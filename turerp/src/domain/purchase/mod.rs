//! Purchase domain module

pub mod model;
pub mod repository;
pub mod service;

// Re-exports
pub use model::{
    CreateGoodsReceipt, CreateGoodsReceiptLine, CreatePurchaseOrder, CreatePurchaseOrderLine,
    CreatePurchaseRequest, CreatePurchaseRequestLine, GoodsReceipt, GoodsReceiptLine,
    GoodsReceiptResponse, GoodsReceiptStatus, PurchaseOrder, PurchaseOrderLine,
    PurchaseOrderResponse, PurchaseOrderStatus, PurchaseRequest, PurchaseRequestLine,
    PurchaseRequestResponse, PurchaseRequestStatus, UpdatePurchaseRequest,
    UpdatePurchaseRequestLine,
};
pub use repository::{
    BoxGoodsReceiptLineRepository, BoxGoodsReceiptRepository, BoxPurchaseOrderLineRepository,
    BoxPurchaseOrderRepository, BoxPurchaseRequestLineRepository, BoxPurchaseRequestRepository,
    GoodsReceiptLineRepository, GoodsReceiptRepository, InMemoryGoodsReceiptLineRepository,
    InMemoryGoodsReceiptRepository, InMemoryPurchaseOrderLineRepository,
    InMemoryPurchaseOrderRepository, InMemoryPurchaseRequestLineRepository,
    InMemoryPurchaseRequestRepository, PurchaseOrderLineRepository, PurchaseOrderRepository,
    PurchaseRequestLineRepository, PurchaseRequestRepository,
};
pub use service::PurchaseService;
