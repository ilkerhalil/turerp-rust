//! Vendor Portal domain module
//!
//! Self-service portal for vendors to view purchase orders, invoices, payments,
//! and manage delivery notes.

pub mod model;
pub mod repository;
pub mod service;

pub use model::*;
pub use repository::{
    BoxDeliveryNoteRepository, BoxVendorUserRepository, DeliveryNoteRepository,
    InMemoryDeliveryNoteRepository, InMemoryVendorUserRepository, VendorUserRepository,
};
pub use service::{BoxVendorPortal, VendorPortal, VendorPortalService};
