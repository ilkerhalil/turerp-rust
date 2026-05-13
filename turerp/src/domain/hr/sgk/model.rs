//! SGK (Social Security Institution) domain models for Turkish payroll integration

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Marital status for AGI (asgari geçim indirimi) calculation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub enum MaritalStatus {
    Single,
    Married,
    Widowed,
    Divorced,
}

impl std::fmt::Display for MaritalStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MaritalStatus::Single => write!(f, "Single"),
            MaritalStatus::Married => write!(f, "Married"),
            MaritalStatus::Widowed => write!(f, "Widowed"),
            MaritalStatus::Divorced => write!(f, "Divorced"),
        }
    }
}

impl std::str::FromStr for MaritalStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Single" => Ok(MaritalStatus::Single),
            "Married" => Ok(MaritalStatus::Married),
            "Widowed" => Ok(MaritalStatus::Widowed),
            "Divorced" => Ok(MaritalStatus::Divorced),
            _ => Err(format!("Invalid marital status: {}", s)),
        }
    }
}

/// SGK employee registration record (e.g., for e-Bildirge)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SgkEmployeeRegistration {
    pub id: i64,
    pub employee_id: i64,
    pub tenant_id: i64,
    pub tc_kimlik_no: String,
    pub sgk_sicil_no: String,
    pub workplace_code: String,
    pub profession_code: String,
    pub registration_date: DateTime<Utc>,
    pub termination_date: Option<DateTime<Utc>>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl crate::common::SoftDeletable for SgkEmployeeRegistration {
    fn is_deleted(&self) -> bool {
        false
    }
    fn deleted_at(&self) -> Option<DateTime<Utc>> {
        None
    }
    fn deleted_by(&self) -> Option<i64> {
        None
    }
    fn mark_deleted(&mut self, _by_user_id: i64) {}
    fn restore(&mut self) {}
}

/// SGK configuration for a given year (rates, ceilings, AGI)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SgkConfig {
    pub id: i64,
    pub tenant_id: i64,
    pub year: i32,
    pub min_wage: Decimal,
    pub sgk_earnings_ceiling: Decimal,
    pub sgk_worker_rate: Decimal,
    pub unemployment_worker_rate: Decimal,
    pub stamp_tax_rate: Decimal,
    pub agi_amount_single: Decimal,
    pub agi_amount_married: Decimal,
    pub agi_per_child: Decimal,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Income tax bracket for a given year
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct IncomeTaxBracket {
    pub id: i64,
    pub year: i32,
    pub bracket_no: i32,
    pub lower_limit: Decimal,
    pub upper_limit: Option<Decimal>,
    pub rate: Decimal,
}

/// Employee bonus record (performance, holiday, year-end, etc.)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EmployeeBonus {
    pub id: i64,
    pub employee_id: i64,
    pub tenant_id: i64,
    pub bonus_type: String,
    pub amount: Decimal,
    pub bonus_month: i32,
    pub bonus_year: i32,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl crate::common::SoftDeletable for EmployeeBonus {
    fn is_deleted(&self) -> bool {
        false
    }
    fn deleted_at(&self) -> Option<DateTime<Utc>> {
        None
    }
    fn deleted_by(&self) -> Option<i64> {
        None
    }
    fn mark_deleted(&mut self, _by_user_id: i64) {}
    fn restore(&mut self) {}
}

/// Per-employee SGK payroll line item
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SgkPayrollLineItem {
    pub employee_id: i64,
    pub gross_salary: Decimal,
    pub sgk_earnings_base: Decimal,
    pub sgk_premium_worker: Decimal,
    pub unemployment_premium_worker: Decimal,
    pub income_tax_base: Decimal,
    pub income_tax: Decimal,
    pub stamp_tax: Decimal,
    pub agi: Decimal,
    pub net_salary: Decimal,
    pub employer_cost: Decimal,
}

/// SGK payroll summary for a given period
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SgkPayrollSummary {
    pub tenant_id: i64,
    pub period_year: i32,
    pub period_month: i32,
    pub total_gross: Decimal,
    pub total_sgk_premium_worker: Decimal,
    pub total_unemployment_worker: Decimal,
    pub total_income_tax: Decimal,
    pub total_stamp_tax: Decimal,
    pub total_agi: Decimal,
    pub total_net: Decimal,
    pub total_employer_cost: Decimal,
    pub employee_count: i32,
    pub line_items: Vec<SgkPayrollLineItem>,
}

/// Create SGK employee registration request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateSgkEmployeeRegistration {
    pub employee_id: i64,
    pub tenant_id: i64,
    pub tc_kimlik_no: String,
    pub sgk_sicil_no: String,
    pub workplace_code: String,
    pub profession_code: String,
    pub registration_date: DateTime<Utc>,
}

impl CreateSgkEmployeeRegistration {
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        if self.tc_kimlik_no.trim().is_empty() {
            errors.push("TC Kimlik No is required".to_string());
        }
        if self.sgk_sicil_no.trim().is_empty() {
            errors.push("SGK sicil number is required".to_string());
        }
        if self.workplace_code.trim().is_empty() {
            errors.push("Workplace code is required".to_string());
        }
        if self.profession_code.trim().is_empty() {
            errors.push("Profession code is required".to_string());
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// Create SGK config request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateSgkConfig {
    pub tenant_id: i64,
    pub year: i32,
    pub min_wage: Decimal,
    pub sgk_earnings_ceiling: Decimal,
    pub sgk_worker_rate: Decimal,
    pub unemployment_worker_rate: Decimal,
    pub stamp_tax_rate: Decimal,
    pub agi_amount_single: Decimal,
    pub agi_amount_married: Decimal,
    pub agi_per_child: Decimal,
}

impl CreateSgkConfig {
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        if self.year < 2000 || self.year > 2100 {
            errors.push("Year must be between 2000 and 2100".to_string());
        }
        if self.min_wage < Decimal::ZERO {
            errors.push("Minimum wage cannot be negative".to_string());
        }
        if self.sgk_earnings_ceiling < Decimal::ZERO {
            errors.push("SGK earnings ceiling cannot be negative".to_string());
        }
        if self.sgk_worker_rate < Decimal::ZERO {
            errors.push("SGK worker rate cannot be negative".to_string());
        }
        if self.unemployment_worker_rate < Decimal::ZERO {
            errors.push("Unemployment worker rate cannot be negative".to_string());
        }
        if self.stamp_tax_rate < Decimal::ZERO {
            errors.push("Stamp tax rate cannot be negative".to_string());
        }
        if self.agi_amount_single < Decimal::ZERO {
            errors.push("AGI single amount cannot be negative".to_string());
        }
        if self.agi_amount_married < Decimal::ZERO {
            errors.push("AGI married amount cannot be negative".to_string());
        }
        if self.agi_per_child < Decimal::ZERO {
            errors.push("AGI per child amount cannot be negative".to_string());
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// Create employee bonus request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateEmployeeBonus {
    pub employee_id: i64,
    pub tenant_id: i64,
    pub bonus_type: String,
    pub amount: Decimal,
    pub bonus_month: i32,
    pub bonus_year: i32,
    pub description: Option<String>,
}

impl CreateEmployeeBonus {
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        if self.bonus_type.trim().is_empty() {
            errors.push("Bonus type is required".to_string());
        }
        if self.amount < Decimal::ZERO {
            errors.push("Bonus amount cannot be negative".to_string());
        }
        if self.bonus_month < 1 || self.bonus_month > 12 {
            errors.push("Bonus month must be between 1 and 12".to_string());
        }
        if self.bonus_year < 2000 || self.bonus_year > 2100 {
            errors.push("Bonus year must be between 2000 and 2100".to_string());
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// Update SGK config request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateSgkConfig {
    pub min_wage: Option<Decimal>,
    pub sgk_earnings_ceiling: Option<Decimal>,
    pub sgk_worker_rate: Option<Decimal>,
    pub unemployment_worker_rate: Option<Decimal>,
    pub stamp_tax_rate: Option<Decimal>,
    pub agi_amount_single: Option<Decimal>,
    pub agi_amount_married: Option<Decimal>,
    pub agi_per_child: Option<Decimal>,
}
