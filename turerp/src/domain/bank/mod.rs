//! Bank integration module for Turkish banks

pub mod adapter;
pub mod camt_parser;
pub mod model;
pub mod postgres_repository;
pub mod repository;
pub mod service;

// Re-exports
pub use adapter::{
    BankAdapter, BankAdapterFactory, BoxBankAdapter, MockGenericAdapter, MockHalkbankAdapter,
    MockIsBankasiAdapter, MockZiraatAdapter,
};
pub use camt_parser::parse_camt053;
pub use model::{
    BankAccount, BankAccountResponse, BankApiCredentials, BankCode, BankConnectionStatus,
    BankStatement, BankTransaction, BankTransactionResponse, CamtEntry, CamtStatement,
    CheckPaymentStatus, CreateBankAccount, CreateReconciliationRule, ImportBankStatement,
    MatchField, MatchStatus, MatchTransaction, ParsedBankTransaction, PaymentInitiation,
    PaymentInitiationResponse, PaymentStatus, PaymentType, ReconciliationReport,
    ReconciliationRule, StatementFormat, UpdateBankAccount, UpdateReconciliationRule,
};
pub use postgres_repository::PostgresBankRepository;
pub use repository::{BankRepository, BoxBankRepository, InMemoryBankRepository};
pub use service::BankService;
