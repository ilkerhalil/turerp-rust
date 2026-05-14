//! Barcode domain module

pub mod model;
pub mod repository;
pub mod service;

pub use model::{
    BarcodeConfig, BarcodeResponse, BarcodeType, CreateBarcode, GenerateBarcodeRequest,
};
pub use repository::{BarcodeRepository, BoxBarcodeRepository, InMemoryBarcodeRepository};
pub use service::BarcodeService;
