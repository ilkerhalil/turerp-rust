//! Blockchain ledger sub-module for e-Defter
//!
//! Provides cryptographic hash chain and Merkle tree functionality
//! for Turkish e-Defter (electronic ledger) compliance.

pub mod model;
pub mod postgres_repository;
pub mod repository;
pub mod service;

pub use model::*;
pub use postgres_repository::PostgresBlockchainLedgerRepository;
pub use repository::{
    BlockchainLedgerRepository, BoxBlockchainLedgerRepository, InMemoryBlockchainLedgerRepository,
};
pub use service::BlockchainLedgerService;
