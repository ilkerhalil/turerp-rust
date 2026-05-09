//! Bank integration module for Turkish banks

pub mod model;
#[cfg(feature = "postgres")]
pub mod postgres_repository;
pub mod repository;
pub mod service;

// Re-exports
pub use model::{
    BankAccount, BankAccountResponse, BankCode, BankStatement, BankTransaction,
    BankTransactionResponse, CreateBankAccount, CreateReconciliationRule, ImportBankStatement,
    MatchField, MatchStatus, MatchTransaction, ParsedBankTransaction, ReconciliationReport,
    ReconciliationRule, StatementFormat, UpdateBankAccount, UpdateReconciliationRule,
};
#[cfg(feature = "postgres")]
pub use postgres_repository::PostgresBankRepository;
pub use repository::{BankRepository, BoxBankRepository, InMemoryBankRepository};
pub use service::BankService;
