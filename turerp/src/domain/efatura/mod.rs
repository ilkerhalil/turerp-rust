//! e-Fatura domain module
//!
//! Provides Turkish e-Fatura (electronic invoicing) integration with GIB
//! (Gelir İdaresi Başkanlığı), including UBL-TR document management,
//! signing, sending, and status tracking.

pub mod model;
pub mod repository;
pub mod service;
pub mod ubl;

#[cfg(feature = "postgres")]
pub mod postgres_repository;

// Re-exports
pub use model::{
    AddressInfo, CreateEFatura, EFatura, EFaturaLine, EFaturaProfile, EFaturaResponse,
    EFaturaStatus, MonetaryTotal, PartyInfo, TaxSubtotal,
};
pub use repository::{BoxEFaturaRepository, EFaturaRepository, InMemoryEFaturaRepository};
pub use service::EFaturaService;
pub use ubl::{
    efatura_to_ubl_xml, ubl_xml_to_efatura_partial, validate_efatura, validate_ubl_xml,
    UblPartialInvoice, ValidationResult,
};
