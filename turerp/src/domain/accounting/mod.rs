//! Accounting domain module

pub mod model;
#[cfg(feature = "postgres")]
pub mod postgres_repository;
pub mod repository;
pub mod service;

// Re-exports
pub use model::{
    Account, AccountBalance, AccountSubType, AccountType, CreateAccount, CreateJournalEntry,
    CreateJournalLine, JournalEntry, JournalEntryStatus, JournalLine, TrialBalance,
};
#[cfg(feature = "postgres")]
pub use postgres_repository::{
    PostgresAccountRepository, PostgresJournalEntryRepository, PostgresJournalLineRepository,
};
pub use repository::{
    AccountRepository, BoxAccountRepository, BoxJournalEntryRepository, BoxJournalLineRepository,
    InMemoryAccountRepository, InMemoryJournalEntryRepository, InMemoryJournalLineRepository,
    JournalEntryRepository, JournalLineRepository,
};
pub use service::AccountingService;
