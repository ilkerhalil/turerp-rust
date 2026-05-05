//! e-Defter service -- business logic for Turkish electronic ledger management
//!
//! Orchestrates e-Defter period lifecycle: creation, population from accounting,
//! balance validation, GIB XML generation, berat signing, and saklayici submission.

use chrono::Utc;
use rust_decimal::Decimal;

use crate::common::pagination::{PaginatedResult, PaginationParams};
use crate::domain::edefter::gib;
use crate::domain::edefter::model::*;
use crate::domain::edefter::repository::BoxEDefterRepository;
use crate::error::ApiError;

/// Service for managing e-Defter periods and GIB integration
pub struct EDefterService {
    repo: BoxEDefterRepository,
}

impl EDefterService {
    pub fn new(repo: BoxEDefterRepository) -> Self {
        Self { repo }
    }

    /// Create a new ledger period
    pub async fn create_period(
        &self,
        create: CreateLedgerPeriod,
        tenant_id: i64,
    ) -> Result<LedgerPeriod, ApiError> {
        let period = LedgerPeriod {
            id: 0,
            tenant_id,
            year: create.year,
            month: create.month,
            period_type: create.period_type,
            status: EDefterStatus::Draft,
            berat_signed_at: None,
            sent_at: None,
            created_at: Utc::now(),
        };
        self.repo.create_period(period).await
    }

    /// Get a ledger period by ID
    pub async fn get_period(&self, id: i64, tenant_id: i64) -> Result<LedgerPeriod, ApiError> {
        self.repo
            .find_period_by_id(id, tenant_id)
            .await?
            .ok_or_else(|| ApiError::NotFound(format!("Ledger period {} not found", id)))
    }

    /// List ledger periods with optional year/type filters and pagination
    pub async fn list_periods(
        &self,
        tenant_id: i64,
        year: Option<i32>,
        period_type: Option<LedgerType>,
        params: PaginationParams,
    ) -> Result<PaginatedResult<LedgerPeriod>, ApiError> {
        self.repo
            .find_periods(tenant_id, year, period_type, params)
            .await
    }

    /// Populate a ledger period from accounting entries.
    ///
    /// Accepts a list of YevmiyeEntry items to add to the period.
    /// This is the cross-module integration point between Accounting and e-Defter:
    /// the API layer calls this with data derived from journal entries.
    pub async fn populate_from_accounting(
        &self,
        period_id: i64,
        tenant_id: i64,
        entries: Vec<YevmiyeEntry>,
    ) -> Result<Vec<YevmiyeEntry>, ApiError> {
        // Validate the period exists and belongs to this tenant
        let period = self.get_period(period_id, tenant_id).await?;

        if period.status != EDefterStatus::Draft {
            return Err(ApiError::BadRequest(format!(
                "Cannot populate period in {} status; must be Draft",
                period.status
            )));
        }

        let mut populated = Vec::new();
        for entry in entries {
            let entry = YevmiyeEntry { period_id, ..entry };
            let stored = self.repo.add_entry(entry).await?;
            populated.push(stored);
        }

        Ok(populated)
    }

    /// Validate that debit and credit totals balance for a period's entries
    pub async fn validate_balance(
        &self,
        period_id: i64,
        tenant_id: i64,
    ) -> Result<BalanceCheckResult, ApiError> {
        let _period = self.get_period(period_id, tenant_id).await?;
        let entries = self.repo.find_entries(period_id).await?;

        let total_debit: Decimal = entries.iter().map(|e| e.debit_total).sum();
        let total_credit: Decimal = entries.iter().map(|e| e.credit_total).sum();
        let difference = (total_debit - total_credit).abs();

        let is_balanced = difference == Decimal::ZERO;
        let mut errors = Vec::new();
        if !is_balanced {
            errors.push(format!(
                "Debit total ({}) does not equal credit total ({})",
                total_debit, total_credit
            ));
        }

        Ok(BalanceCheckResult {
            is_balanced,
            total_debit,
            total_credit,
            difference,
            errors,
        })
    }

    /// Generate GIB-format Yevmiye defteri XML for a period
    pub async fn generate_yevmiye_xml(
        &self,
        period_id: i64,
        tenant_id: i64,
    ) -> Result<String, ApiError> {
        let period = self.get_period(period_id, tenant_id).await?;
        let entries = self.repo.find_entries(period_id).await?;

        gib::generate_yevmiye_xml(&period, &entries)
            .map_err(|e| ApiError::Internal(format!("Yevmiye XML generation failed: {}", e)))
    }

    /// Generate GIB-format Buyuk defter XML for a period
    pub async fn generate_buyuk_defter_xml(
        &self,
        period_id: i64,
        tenant_id: i64,
    ) -> Result<String, ApiError> {
        let period = self.get_period(period_id, tenant_id).await?;
        let entries = self.repo.find_entries(period_id).await?;

        gib::generate_buyuk_defter_xml(&period, &entries)
            .map_err(|e| ApiError::Internal(format!("Buyuk defter XML generation failed: {}", e)))
    }

    /// Sign a berat (certificate) for a period.
    ///
    /// Creates a mock BeratInfo and updates the period status to Signed.
    pub async fn sign_berat(&self, period_id: i64, tenant_id: i64) -> Result<BeratInfo, ApiError> {
        let period = self.get_period(period_id, tenant_id).await?;

        if period.status != EDefterStatus::Draft {
            return Err(ApiError::BadRequest(format!(
                "Cannot sign period in {} status; must be Draft",
                period.status
            )));
        }

        let now = Utc::now();
        let berat = BeratInfo {
            period_id,
            serial_number: format!("BERAT-{}-{}", period.year, period.month),
            sign_time: now,
            signer: "e-Defter System".to_string(),
            digest_value: format!("DIGEST-{}", now.timestamp_millis()),
            signature_value: format!("SIG-{}", now.timestamp_millis()),
        };

        self.repo.update_berat(period_id, berat.clone()).await?;
        self.repo
            .update_period_status(period_id, tenant_id, EDefterStatus::Signed)
            .await?;

        Ok(berat)
    }

    /// Send a signed ledger period to the saklayici (archiver).
    ///
    /// Placeholder for actual GIB integration; updates status to Sent.
    pub async fn send_to_saklayici(
        &self,
        period_id: i64,
        tenant_id: i64,
    ) -> Result<LedgerPeriod, ApiError> {
        let period = self.get_period(period_id, tenant_id).await?;

        if period.status != EDefterStatus::Signed {
            return Err(ApiError::BadRequest(format!(
                "Cannot send period in {} status; must be Signed",
                period.status
            )));
        }

        self.repo
            .update_period_status(period_id, tenant_id, EDefterStatus::Sent)
            .await
    }

    /// Check the current status of a ledger period
    pub async fn check_status(
        &self,
        period_id: i64,
        tenant_id: i64,
    ) -> Result<EDefterStatus, ApiError> {
        let period = self.get_period(period_id, tenant_id).await?;
        Ok(period.status)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::edefter::repository::InMemoryEDefterRepository;
    use std::sync::Arc;

    fn make_service() -> EDefterService {
        let repo = Arc::new(InMemoryEDefterRepository::new()) as BoxEDefterRepository;
        EDefterService::new(repo)
    }

    fn sample_create() -> CreateLedgerPeriod {
        CreateLedgerPeriod {
            year: 2024,
            month: 6,
            period_type: LedgerType::YevmiyeDefteri,
        }
    }

    #[tokio::test]
    async fn test_create_period() {
        let svc = make_service();
        let period = svc.create_period(sample_create(), 1).await.unwrap();
        assert_eq!(period.tenant_id, 1);
        assert_eq!(period.year, 2024);
        assert_eq!(period.month, 6);
        assert_eq!(period.status, EDefterStatus::Draft);
    }

    #[tokio::test]
    async fn test_get_period() {
        let svc = make_service();
        let created = svc.create_period(sample_create(), 1).await.unwrap();

        let found = svc.get_period(created.id, 1).await.unwrap();
        assert_eq!(found.id, created.id);

        let result = svc.get_period(created.id, 999).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_list_periods() {
        let svc = make_service();
        svc.create_period(sample_create(), 1).await.unwrap();

        let mut create2 = sample_create();
        create2.year = 2023;
        create2.period_type = LedgerType::BuyukDefter;
        svc.create_period(create2, 1).await.unwrap();

        let all = svc
            .list_periods(1, None, None, PaginationParams::default())
            .await
            .unwrap();
        assert_eq!(all.items.len(), 2);

        let by_year = svc
            .list_periods(1, Some(2024), None, PaginationParams::default())
            .await
            .unwrap();
        assert_eq!(by_year.items.len(), 1);

        let by_type = svc
            .list_periods(
                1,
                None,
                Some(LedgerType::BuyukDefter),
                PaginationParams::default(),
            )
            .await
            .unwrap();
        assert_eq!(by_type.items.len(), 1);

        let other_tenant = svc
            .list_periods(999, None, None, PaginationParams::default())
            .await
            .unwrap();
        assert_eq!(other_tenant.items.len(), 0);
    }

    #[tokio::test]
    async fn test_populate_from_accounting() {
        let svc = make_service();
        let created = svc.create_period(sample_create(), 1).await.unwrap();

        let entries = vec![YevmiyeEntry {
            id: 0,
            period_id: created.id,
            entry_number: 1,
            entry_date: chrono::NaiveDate::from_ymd_opt(2024, 6, 15).unwrap(),
            explanation: "From accounting".to_string(),
            debit_total: Decimal::new(5000, 2),
            credit_total: Decimal::new(5000, 2),
            lines: vec![YevmiyeLine {
                account_code: "100.01".to_string(),
                account_name: "Kasa".to_string(),
                debit: Decimal::new(5000, 2),
                credit: Decimal::ZERO,
                explanation: "Accounting entry".to_string(),
            }],
        }];

        let result = svc
            .populate_from_accounting(created.id, 1, entries)
            .await
            .unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].explanation, "From accounting");
    }

    #[tokio::test]
    async fn test_populate_from_accounting_rejects_non_draft() {
        let svc = make_service();
        let created = svc.create_period(sample_create(), 1).await.unwrap();

        // Sign the period so it is no longer Draft
        svc.sign_berat(created.id, 1).await.unwrap();

        let entries = vec![YevmiyeEntry {
            id: 0,
            period_id: created.id,
            entry_number: 1,
            entry_date: chrono::NaiveDate::from_ymd_opt(2024, 6, 15).unwrap(),
            explanation: "Should fail".to_string(),
            debit_total: Decimal::ZERO,
            credit_total: Decimal::ZERO,
            lines: vec![],
        }];

        let result = svc.populate_from_accounting(created.id, 1, entries).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_populate_from_accounting_invalid_period() {
        let svc = make_service();

        let entries = vec![YevmiyeEntry {
            id: 0,
            period_id: 9999,
            entry_number: 1,
            entry_date: chrono::NaiveDate::from_ymd_opt(2024, 6, 15).unwrap(),
            explanation: "Invalid".to_string(),
            debit_total: Decimal::ZERO,
            credit_total: Decimal::ZERO,
            lines: vec![],
        }];

        let result = svc.populate_from_accounting(9999, 1, entries).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_validate_balance_empty() {
        let svc = make_service();
        let created = svc.create_period(sample_create(), 1).await.unwrap();

        let result = svc.validate_balance(created.id, 1).await.unwrap();
        assert!(result.is_balanced);
        assert_eq!(result.total_debit, Decimal::ZERO);
        assert_eq!(result.total_credit, Decimal::ZERO);
    }

    #[tokio::test]
    async fn test_validate_balance_with_entries() {
        let svc = make_service();
        let created = svc.create_period(sample_create(), 1).await.unwrap();

        // Add a balanced entry
        let entry = YevmiyeEntry {
            id: 0,
            period_id: created.id,
            entry_number: 1,
            entry_date: chrono::NaiveDate::from_ymd_opt(2024, 6, 15).unwrap(),
            explanation: "Test entry".to_string(),
            debit_total: Decimal::new(10000, 2),
            credit_total: Decimal::new(10000, 2),
            lines: vec![
                YevmiyeLine {
                    account_code: "100.01".to_string(),
                    account_name: "Kasa".to_string(),
                    debit: Decimal::new(10000, 2),
                    credit: Decimal::ZERO,
                    explanation: "Kasa borc".to_string(),
                },
                YevmiyeLine {
                    account_code: "600.01".to_string(),
                    account_name: "Banka".to_string(),
                    debit: Decimal::ZERO,
                    credit: Decimal::new(10000, 2),
                    explanation: "Banka alacak".to_string(),
                },
            ],
        };
        svc.repo.add_entry(entry).await.unwrap();

        let result = svc.validate_balance(created.id, 1).await.unwrap();
        assert!(result.is_balanced);
        assert_eq!(result.total_debit, Decimal::new(10000, 2));
        assert_eq!(result.total_credit, Decimal::new(10000, 2));
    }

    #[tokio::test]
    async fn test_validate_balance_unbalanced() {
        let svc = make_service();
        let created = svc.create_period(sample_create(), 1).await.unwrap();

        let entry = YevmiyeEntry {
            id: 0,
            period_id: created.id,
            entry_number: 1,
            entry_date: chrono::NaiveDate::from_ymd_opt(2024, 6, 15).unwrap(),
            explanation: "Unbalanced entry".to_string(),
            debit_total: Decimal::new(10000, 2),
            credit_total: Decimal::new(9000, 2),
            lines: vec![],
        };
        svc.repo.add_entry(entry).await.unwrap();

        let result = svc.validate_balance(created.id, 1).await.unwrap();
        assert!(!result.is_balanced);
        assert!(!result.errors.is_empty());
    }

    #[tokio::test]
    async fn test_generate_yevmiye_xml_empty() {
        let svc = make_service();
        let created = svc.create_period(sample_create(), 1).await.unwrap();

        let result = svc.generate_yevmiye_xml(created.id, 1).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_generate_yevmiye_xml_with_entries() {
        let svc = make_service();
        let created = svc.create_period(sample_create(), 1).await.unwrap();

        let entry = YevmiyeEntry {
            id: 0,
            period_id: created.id,
            entry_number: 1,
            entry_date: chrono::NaiveDate::from_ymd_opt(2024, 6, 15).unwrap(),
            explanation: "Test entry".to_string(),
            debit_total: Decimal::new(10000, 2),
            credit_total: Decimal::new(10000, 2),
            lines: vec![YevmiyeLine {
                account_code: "100.01".to_string(),
                account_name: "Kasa".to_string(),
                debit: Decimal::new(10000, 2),
                credit: Decimal::ZERO,
                explanation: "Kasa borc".to_string(),
            }],
        };
        svc.repo.add_entry(entry).await.unwrap();

        let xml = svc.generate_yevmiye_xml(created.id, 1).await.unwrap();
        assert!(xml.contains("YevmiyeDefteri"));
    }

    #[tokio::test]
    async fn test_generate_buyuk_defter_xml_with_entries() {
        let svc = make_service();
        let created = svc.create_period(sample_create(), 1).await.unwrap();

        let entry = YevmiyeEntry {
            id: 0,
            period_id: created.id,
            entry_number: 1,
            entry_date: chrono::NaiveDate::from_ymd_opt(2024, 6, 15).unwrap(),
            explanation: "Test entry".to_string(),
            debit_total: Decimal::new(10000, 2),
            credit_total: Decimal::new(10000, 2),
            lines: vec![YevmiyeLine {
                account_code: "100.01".to_string(),
                account_name: "Kasa".to_string(),
                debit: Decimal::new(10000, 2),
                credit: Decimal::ZERO,
                explanation: "Kasa borc".to_string(),
            }],
        };
        svc.repo.add_entry(entry).await.unwrap();

        let xml = svc.generate_buyuk_defter_xml(created.id, 1).await.unwrap();
        assert!(xml.contains("BuyukDefter"));
    }

    #[tokio::test]
    async fn test_sign_berat() {
        let svc = make_service();
        let created = svc.create_period(sample_create(), 1).await.unwrap();

        let berat = svc.sign_berat(created.id, 1).await.unwrap();
        assert_eq!(berat.period_id, created.id);
        assert!(berat.serial_number.starts_with("BERAT-"));

        let period = svc.get_period(created.id, 1).await.unwrap();
        assert_eq!(period.status, EDefterStatus::Signed);
    }

    #[tokio::test]
    async fn test_sign_berat_non_draft_fails() {
        let svc = make_service();
        let created = svc.create_period(sample_create(), 1).await.unwrap();

        svc.sign_berat(created.id, 1).await.unwrap();

        let result = svc.sign_berat(created.id, 1).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_send_to_saklayici() {
        let svc = make_service();
        let created = svc.create_period(sample_create(), 1).await.unwrap();

        svc.sign_berat(created.id, 1).await.unwrap();

        let sent = svc.send_to_saklayici(created.id, 1).await.unwrap();
        assert_eq!(sent.status, EDefterStatus::Sent);
    }

    #[tokio::test]
    async fn test_send_to_saklayici_not_signed_fails() {
        let svc = make_service();
        let created = svc.create_period(sample_create(), 1).await.unwrap();

        let result = svc.send_to_saklayici(created.id, 1).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_check_status() {
        let svc = make_service();
        let created = svc.create_period(sample_create(), 1).await.unwrap();

        let status = svc.check_status(created.id, 1).await.unwrap();
        assert_eq!(status, EDefterStatus::Draft);
    }

    #[tokio::test]
    async fn test_tenant_isolation() {
        let svc = make_service();
        let created = svc.create_period(sample_create(), 1).await.unwrap();

        let result = svc.get_period(created.id, 999).await;
        assert!(result.is_err());
    }
}
