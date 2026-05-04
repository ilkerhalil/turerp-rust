//! Chart of Accounts domain model

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::domain::accounting::model::AccountType;

/// Account group (Turkish UMS layout)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum AccountGroup {
    DonenVarliklar,
    DuranVarliklar,
    KisaVadeliYabanciKaynaklar,
    UzunVadeliYabanciKaynaklar,
    OzKaynaklar,
    GelirTablosu,
    GiderTablosu,
}

impl std::fmt::Display for AccountGroup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AccountGroup::DonenVarliklar => write!(f, "DonenVarliklar"),
            AccountGroup::DuranVarliklar => write!(f, "DuranVarliklar"),
            AccountGroup::KisaVadeliYabanciKaynaklar => write!(f, "KisaVadeliYabanciKaynaklar"),
            AccountGroup::UzunVadeliYabanciKaynaklar => write!(f, "UzunVadeliYabanciKaynaklar"),
            AccountGroup::OzKaynaklar => write!(f, "OzKaynaklar"),
            AccountGroup::GelirTablosu => write!(f, "GelirTablosu"),
            AccountGroup::GiderTablosu => write!(f, "GiderTablosu"),
        }
    }
}

impl std::str::FromStr for AccountGroup {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "DonenVarliklar" => Ok(AccountGroup::DonenVarliklar),
            "DuranVarliklar" => Ok(AccountGroup::DuranVarliklar),
            "KisaVadeliYabanciKaynaklar" => Ok(AccountGroup::KisaVadeliYabanciKaynaklar),
            "UzunVadeliYabanciKaynaklar" => Ok(AccountGroup::UzunVadeliYabanciKaynaklar),
            "OzKaynaklar" => Ok(AccountGroup::OzKaynaklar),
            "GelirTablosu" => Ok(AccountGroup::GelirTablosu),
            "GiderTablosu" => Ok(AccountGroup::GiderTablosu),
            _ => Err(format!("Invalid account group: {}", s)),
        }
    }
}

/// Chart of Account entity
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ChartAccount {
    pub id: i64,
    pub tenant_id: i64,
    pub code: String,
    pub name: String,
    pub group: AccountGroup,
    pub parent_code: Option<String>,
    pub level: u8,
    pub account_type: AccountType,
    pub is_active: bool,
    pub balance: Decimal,
    pub allow_posting: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub deleted_by: Option<i64>,
}

impl crate::common::SoftDeletable for ChartAccount {
    fn is_deleted(&self) -> bool {
        self.deleted_at.is_some()
    }
    fn deleted_at(&self) -> Option<DateTime<Utc>> {
        self.deleted_at
    }
    fn deleted_by(&self) -> Option<i64> {
        self.deleted_by
    }
    fn mark_deleted(&mut self, by_user_id: i64) {
        self.deleted_at = Some(Utc::now());
        self.deleted_by = Some(by_user_id);
    }
    fn restore(&mut self) {
        self.deleted_at = None;
        self.deleted_by = None;
    }
}

/// Chart of Account response (without deletion metadata)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ChartAccountResponse {
    pub id: i64,
    pub tenant_id: i64,
    pub code: String,
    pub name: String,
    pub group: AccountGroup,
    pub parent_code: Option<String>,
    pub level: u8,
    pub account_type: AccountType,
    pub is_active: bool,
    pub balance: Decimal,
    pub allow_posting: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

impl From<ChartAccount> for ChartAccountResponse {
    fn from(account: ChartAccount) -> Self {
        Self {
            id: account.id,
            tenant_id: account.tenant_id,
            code: account.code,
            name: account.name,
            group: account.group,
            parent_code: account.parent_code,
            level: account.level,
            account_type: account.account_type,
            is_active: account.is_active,
            balance: account.balance,
            allow_posting: account.allow_posting,
            created_at: account.created_at,
            updated_at: account.updated_at,
        }
    }
}

/// Data for creating a new chart account
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct CreateChartAccount {
    pub code: String,
    pub name: String,
    pub group: AccountGroup,
    pub parent_code: Option<String>,
    pub account_type: AccountType,
    pub allow_posting: bool,
}

/// Data for updating an existing chart account
#[derive(Debug, Clone, Deserialize, Serialize, Default, ToSchema)]
pub struct UpdateChartAccount {
    #[serde(default)]
    pub name: Option<String>,

    #[serde(default)]
    pub group: Option<AccountGroup>,

    #[serde(default)]
    pub is_active: Option<bool>,

    #[serde(default)]
    pub allow_posting: Option<bool>,
}

/// Hierarchical tree node for chart of accounts
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AccountTreeNode {
    pub code: String,
    pub name: String,
    pub group: AccountGroup,
    pub balance: Decimal,
    pub children: Vec<AccountTreeNode>,
}

/// Trial balance entry
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TrialBalanceEntry {
    pub account_code: String,
    pub account_name: String,
    pub debit_balance: Decimal,
    pub credit_balance: Decimal,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::SoftDeletable;
    use std::str::FromStr;

    #[test]
    fn test_account_group_display() {
        assert_eq!(AccountGroup::DonenVarliklar.to_string(), "DonenVarliklar");
        assert_eq!(AccountGroup::DuranVarliklar.to_string(), "DuranVarliklar");
        assert_eq!(
            AccountGroup::KisaVadeliYabanciKaynaklar.to_string(),
            "KisaVadeliYabanciKaynaklar"
        );
        assert_eq!(
            AccountGroup::UzunVadeliYabanciKaynaklar.to_string(),
            "UzunVadeliYabanciKaynaklar"
        );
        assert_eq!(AccountGroup::OzKaynaklar.to_string(), "OzKaynaklar");
        assert_eq!(AccountGroup::GelirTablosu.to_string(), "GelirTablosu");
        assert_eq!(AccountGroup::GiderTablosu.to_string(), "GiderTablosu");
    }

    #[test]
    fn test_account_group_from_str() {
        assert_eq!(
            AccountGroup::from_str("DonenVarliklar").unwrap(),
            AccountGroup::DonenVarliklar
        );
        assert_eq!(
            AccountGroup::from_str("OzKaynaklar").unwrap(),
            AccountGroup::OzKaynaklar
        );
        assert!(AccountGroup::from_str("Invalid").is_err());
    }

    #[test]
    fn test_chart_account_soft_delete() {
        let mut account = ChartAccount {
            id: 1,
            tenant_id: 1,
            code: "100".to_string(),
            name: "Cash".to_string(),
            group: AccountGroup::DonenVarliklar,
            parent_code: None,
            level: 1,
            account_type: AccountType::Asset,
            is_active: true,
            balance: Decimal::ZERO,
            allow_posting: true,
            created_at: Utc::now(),
            updated_at: None,
            deleted_at: None,
            deleted_by: None,
        };

        assert!(!account.is_deleted());
        account.mark_deleted(42);
        assert!(account.is_deleted());
        assert_eq!(account.deleted_by(), Some(42));

        account.restore();
        assert!(!account.is_deleted());
    }

    #[test]
    fn test_chart_account_response_from_chart_account() {
        let account = ChartAccount {
            id: 1,
            tenant_id: 1,
            code: "100".to_string(),
            name: "Cash".to_string(),
            group: AccountGroup::DonenVarliklar,
            parent_code: None,
            level: 1,
            account_type: AccountType::Asset,
            is_active: true,
            balance: Decimal::ZERO,
            allow_posting: true,
            created_at: Utc::now(),
            updated_at: None,
            deleted_at: None,
            deleted_by: None,
        };

        let response: ChartAccountResponse = account.into();
        assert_eq!(response.code, "100");
        assert_eq!(response.name, "Cash");
    }
}
