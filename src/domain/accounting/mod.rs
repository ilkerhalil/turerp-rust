//! Accounting domain module

pub mod model;
pub mod repository;
pub mod service;

// Re-exports
pub use model::{
    Account, AccountBalance, AccountSubType, AccountType, CreateAccount, CreateJournalEntry,
    CreateJournalLine, JournalEntry, JournalEntryStatus, JournalLine, TrialBalance,
};
pub use repository::{
    AccountRepository, BoxAccountRepository, BoxJournalEntryRepository, BoxJournalLineRepository,
    InMemoryAccountRepository, InMemoryJournalEntryRepository, InMemoryJournalLineRepository,
    JournalEntryRepository, JournalLineRepository,
};
pub use service::AccountingService;
