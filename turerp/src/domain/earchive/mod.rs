//! E-Archive domain module
//!
//! Provides Turkish e-Arşiv Fatura (E-Archive Invoice) and E-Serbest Meslek Makbuzu
//! (Freelance Profession Receipt) integration with GİB
//! (Gelir İdaresi Başkanlığı), including UBL-TR document management,
//! signing, sending, and status tracking.

pub mod model;
pub mod repository;
pub mod service;

// Re-exports
pub use model::{
    CreateEarchiveDocument, EarchiveDocument, EarchiveResponse, EarchiveStatus, EarchiveType,
    GenerateEarchiveRequest,
};
pub use repository::{BoxEarchiveRepository, EarchiveRepository, InMemoryEarchiveRepository};
pub use service::EarchiveService;
