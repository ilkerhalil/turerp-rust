//! e-Defter repository trait and in-memory implementation

use async_trait::async_trait;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;

use crate::common::pagination::{PaginatedResult, PaginationParams};
use crate::domain::edefter::model::{
    BeratInfo, EDefterStatus, LedgerPeriod, LedgerType, YevmiyeEntry,
};
use crate::error::ApiError;

// ---------------------------------------------------------------------------
// EDefterRepository
// ---------------------------------------------------------------------------

/// Repository trait for e-Defter operations
#[async_trait]
pub trait EDefterRepository: Send + Sync {
    /// Create a new ledger period
    async fn create_period(&self, period: LedgerPeriod) -> Result<LedgerPeriod, ApiError>;

    /// Find a ledger period by ID
    async fn find_period_by_id(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<Option<LedgerPeriod>, ApiError>;

    /// Find ledger periods with optional filters and pagination
    async fn find_periods(
        &self,
        tenant_id: i64,
        year: Option<i32>,
        period_type: Option<LedgerType>,
        params: PaginationParams,
    ) -> Result<PaginatedResult<LedgerPeriod>, ApiError>;

    /// Update the status of a ledger period
    async fn update_period_status(
        &self,
        id: i64,
        tenant_id: i64,
        status: EDefterStatus,
    ) -> Result<LedgerPeriod, ApiError>;

    /// Add a Yevmiye entry
    async fn add_entry(&self, entry: YevmiyeEntry) -> Result<YevmiyeEntry, ApiError>;

    /// Find Yevmiye entries for a period
    async fn find_entries(&self, period_id: i64) -> Result<Vec<YevmiyeEntry>, ApiError>;

    /// Update berat information for a period
    async fn update_berat(&self, period_id: i64, berat: BeratInfo) -> Result<(), ApiError>;

    /// Get berat information for a period
    async fn get_berat(&self, period_id: i64) -> Result<Option<BeratInfo>, ApiError>;
}

/// Type alias for boxed EDefterRepository
pub type BoxEDefterRepository = Arc<dyn EDefterRepository>;

// ---------------------------------------------------------------------------
// InMemoryEDefterRepository
// ---------------------------------------------------------------------------

struct Inner {
    periods: HashMap<i64, LedgerPeriod>,
    entries: HashMap<i64, Vec<YevmiyeEntry>>,
    berats: HashMap<i64, BeratInfo>,
    next_period_id: AtomicI64,
    next_entry_id: AtomicI64,
}

/// In-memory e-Defter repository for testing and development
pub struct InMemoryEDefterRepository {
    inner: Mutex<Inner>,
}

impl InMemoryEDefterRepository {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(Inner {
                periods: HashMap::new(),
                entries: HashMap::new(),
                berats: HashMap::new(),
                next_period_id: AtomicI64::new(1),
                next_entry_id: AtomicI64::new(1),
            }),
        }
    }
}

impl Default for InMemoryEDefterRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl EDefterRepository for InMemoryEDefterRepository {
    async fn create_period(&self, period: LedgerPeriod) -> Result<LedgerPeriod, ApiError> {
        let mut inner = self.inner.lock();
        let id = inner.next_period_id.fetch_add(1, Ordering::SeqCst);
        let now = chrono::Utc::now();

        let stored = LedgerPeriod {
            id,
            tenant_id: period.tenant_id,
            year: period.year,
            month: period.month,
            period_type: period.period_type,
            status: period.status,
            berat_signed_at: period.berat_signed_at,
            sent_at: period.sent_at,
            created_at: now,
        };

        inner.periods.insert(id, stored.clone());
        inner.entries.insert(id, Vec::new());
        Ok(stored)
    }

    async fn find_period_by_id(
        &self,
        id: i64,
        tenant_id: i64,
    ) -> Result<Option<LedgerPeriod>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .periods
            .get(&id)
            .filter(|p| p.tenant_id == tenant_id)
            .cloned())
    }

    async fn find_periods(
        &self,
        tenant_id: i64,
        year: Option<i32>,
        period_type: Option<LedgerType>,
        params: PaginationParams,
    ) -> Result<PaginatedResult<LedgerPeriod>, ApiError> {
        let inner = self.inner.lock();
        let mut items: Vec<LedgerPeriod> = inner
            .periods
            .values()
            .filter(|p| p.tenant_id == tenant_id)
            .filter(|p| match &year {
                Some(y) => p.year == *y,
                None => true,
            })
            .filter(|p| match &period_type {
                Some(pt) => p.period_type == *pt,
                None => true,
            })
            .cloned()
            .collect();

        items.sort_by(|a, b| a.id.cmp(&b.id));
        let total = items.len() as u64;
        let start = (params.page.saturating_sub(1)) * params.per_page;
        let paginated: Vec<LedgerPeriod> = items
            .into_iter()
            .skip(start as usize)
            .take(params.per_page as usize)
            .collect();
        Ok(PaginatedResult::new(
            paginated,
            params.page,
            params.per_page,
            total,
        ))
    }

    async fn update_period_status(
        &self,
        id: i64,
        tenant_id: i64,
        status: EDefterStatus,
    ) -> Result<LedgerPeriod, ApiError> {
        let mut inner = self.inner.lock();

        let period = inner
            .periods
            .get_mut(&id)
            .filter(|p| p.tenant_id == tenant_id)
            .ok_or_else(|| ApiError::NotFound(format!("Ledger period {} not found", id)))?;

        period.status = status;
        Ok(period.clone())
    }

    async fn add_entry(&self, entry: YevmiyeEntry) -> Result<YevmiyeEntry, ApiError> {
        let mut inner = self.inner.lock();
        let id = inner.next_entry_id.fetch_add(1, Ordering::SeqCst);

        // Verify the period exists
        if !inner.periods.contains_key(&entry.period_id) {
            return Err(ApiError::NotFound(format!(
                "Ledger period {} not found",
                entry.period_id
            )));
        }

        let stored = YevmiyeEntry {
            id,
            period_id: entry.period_id,
            entry_number: entry.entry_number,
            entry_date: entry.entry_date,
            explanation: entry.explanation,
            debit_total: entry.debit_total,
            credit_total: entry.credit_total,
            lines: entry.lines,
        };

        if let Some(v) = inner.entries.get_mut(&entry.period_id) {
            v.push(stored.clone());
        }

        Ok(stored)
    }

    async fn find_entries(&self, period_id: i64) -> Result<Vec<YevmiyeEntry>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner.entries.get(&period_id).cloned().unwrap_or_default())
    }

    async fn update_berat(&self, period_id: i64, berat: BeratInfo) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();

        // Verify the period exists
        if !inner.periods.contains_key(&period_id) {
            return Err(ApiError::NotFound(format!(
                "Ledger period {} not found",
                period_id
            )));
        }

        inner.berats.insert(period_id, berat);
        Ok(())
    }

    async fn get_berat(&self, period_id: i64) -> Result<Option<BeratInfo>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner.berats.get(&period_id).cloned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::edefter::model::{EDefterStatus, LedgerType, YevmiyeLine};
    use chrono::NaiveDate;
    use rust_decimal::Decimal;

    fn sample_period(tenant_id: i64) -> LedgerPeriod {
        LedgerPeriod {
            id: 0,
            tenant_id,
            year: 2024,
            month: 6,
            period_type: LedgerType::YevmiyeDefteri,
            status: EDefterStatus::Draft,
            berat_signed_at: None,
            sent_at: None,
            created_at: chrono::Utc::now(),
        }
    }

    fn sample_entry(period_id: i64) -> YevmiyeEntry {
        YevmiyeEntry {
            id: 0,
            period_id,
            entry_number: 1,
            entry_date: NaiveDate::from_ymd_opt(2024, 6, 15).unwrap(),
            explanation: "Test entry".to_string(),
            debit_total: Decimal::new(10000, 2),
            credit_total: Decimal::new(10000, 2),
            lines: vec![YevmiyeLine {
                account_code: "100.01".to_string(),
                account_name: "Kasa".to_string(),
                debit: Decimal::new(10000, 2),
                credit: Decimal::ZERO,
                explanation: "Kasa borç".to_string(),
            }],
        }
    }

    #[tokio::test]
    async fn test_create_and_find_period() {
        let repo = InMemoryEDefterRepository::new();

        let period = repo.create_period(sample_period(1)).await.unwrap();
        assert_eq!(period.id, 1);
        assert_eq!(period.tenant_id, 1);
        assert_eq!(period.status, EDefterStatus::Draft);

        let found = repo.find_period_by_id(1, 1).await.unwrap().unwrap();
        assert_eq!(found.id, period.id);

        // Not found for different tenant
        let not_found = repo.find_period_by_id(1, 999).await.unwrap();
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_find_periods_with_filters() {
        let repo = InMemoryEDefterRepository::new();

        repo.create_period(sample_period(1)).await.unwrap();

        let mut period2 = sample_period(1);
        period2.year = 2023;
        period2.period_type = LedgerType::BuyukDefter;
        repo.create_period(period2).await.unwrap();

        // All periods for tenant
        let all = repo
            .find_periods(1, None, None, PaginationParams::default())
            .await
            .unwrap();
        assert_eq!(all.items.len(), 2);

        // Filter by year
        let filtered = repo
            .find_periods(1, Some(2024), None, PaginationParams::default())
            .await
            .unwrap();
        assert_eq!(filtered.items.len(), 1);
        assert_eq!(filtered.items[0].year, 2024);

        // Filter by type
        let by_type = repo
            .find_periods(
                1,
                None,
                Some(LedgerType::BuyukDefter),
                PaginationParams::default(),
            )
            .await
            .unwrap();
        assert_eq!(by_type.items.len(), 1);

        // Different tenant
        let other = repo
            .find_periods(999, None, None, PaginationParams::default())
            .await
            .unwrap();
        assert_eq!(other.items.len(), 0);
    }

    #[tokio::test]
    async fn test_update_period_status() {
        let repo = InMemoryEDefterRepository::new();

        repo.create_period(sample_period(1)).await.unwrap();

        let updated = repo
            .update_period_status(1, 1, EDefterStatus::Signed)
            .await
            .unwrap();
        assert_eq!(updated.status, EDefterStatus::Signed);

        // Not found for different tenant
        let result = repo.update_period_status(1, 999, EDefterStatus::Sent).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_add_and_find_entries() {
        let repo = InMemoryEDefterRepository::new();

        let period = repo.create_period(sample_period(1)).await.unwrap();
        let entry = repo.add_entry(sample_entry(period.id)).await.unwrap();

        assert_eq!(entry.id, 1);
        assert_eq!(entry.period_id, period.id);

        let entries = repo.find_entries(period.id).await.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].entry_number, 1);
    }

    #[tokio::test]
    async fn test_add_entry_nonexistent_period() {
        let repo = InMemoryEDefterRepository::new();

        let result = repo.add_entry(sample_entry(9999)).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_find_entries_empty() {
        let repo = InMemoryEDefterRepository::new();

        let entries = repo.find_entries(1).await.unwrap();
        assert!(entries.is_empty());
    }

    #[tokio::test]
    async fn test_berat_crud() {
        let repo = InMemoryEDefterRepository::new();

        let period = repo.create_period(sample_period(1)).await.unwrap();

        let berat = BeratInfo {
            period_id: period.id,
            serial_number: "BERAT-001".to_string(),
            sign_time: chrono::Utc::now(),
            signer: "Test Signer".to_string(),
            digest_value: "abc123".to_string(),
            signature_value: "sig456".to_string(),
        };

        repo.update_berat(period.id, berat.clone()).await.unwrap();

        let found = repo.get_berat(period.id).await.unwrap().unwrap();
        assert_eq!(found.serial_number, "BERAT-001");
        assert_eq!(found.signer, "Test Signer");

        // No berat for nonexistent period
        let not_found = repo.get_berat(9999).await.unwrap();
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_berat_nonexistent_period() {
        let repo = InMemoryEDefterRepository::new();

        let berat = BeratInfo {
            period_id: 9999,
            serial_number: "BERAT-001".to_string(),
            sign_time: chrono::Utc::now(),
            signer: "Test Signer".to_string(),
            digest_value: "abc123".to_string(),
            signature_value: "sig456".to_string(),
        };

        let result = repo.update_berat(9999, berat).await;
        assert!(result.is_err());
    }
}
