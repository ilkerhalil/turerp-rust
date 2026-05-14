//! Customer Portal domain module
//!
//! Self-service portal for customers to view orders, invoices, payments,
//! and create support tickets.

pub mod model;
pub mod repository;
pub mod service;

pub use model::*;
pub use repository::{
    BoxPortalUserRepository, BoxSupportTicketRepository, InMemoryPortalUserRepository,
    InMemorySupportTicketRepository, PortalUserRepository, SupportTicketRepository,
};
pub use service::CustomerPortalService;
