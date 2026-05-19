//! Chart of Accounts module

pub mod model;
pub mod repository;
pub mod service;

pub mod postgres_repository;

// Re-exports
pub use model::{
    AccountGroup, AccountTreeNode, ChartAccount, ChartAccountResponse, CreateChartAccount,
    TrialBalanceEntry, UpdateChartAccount,
};
pub use repository::{
    BoxChartAccountRepository, ChartAccountRepository, InMemoryChartAccountRepository,
};

pub use postgres_repository::PostgresChartAccountRepository;

pub use service::ChartOfAccountsService;
