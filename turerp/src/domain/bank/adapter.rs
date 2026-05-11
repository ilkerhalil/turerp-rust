//! Bank API adapters for Turkish banks
//!
//! Provides a unified trait for interacting with Turkish bank APIs,
//! with mock implementations for testing.

use async_trait::async_trait;
use chrono::{NaiveDate, Utc};
use rust_decimal::Decimal;
use std::sync::Arc;

use crate::domain::bank::model::{
    BankApiCredentials, BankCode, BankConnectionStatus, CamtStatement, PaymentInitiation,
    PaymentInitiationResponse, PaymentStatus,
};
use crate::error::ApiError;

use std::collections::HashMap;
use std::sync::LazyLock;

static SHARED_PAYMENT_STATUSES: LazyLock<parking_lot::Mutex<HashMap<String, PaymentStatus>>> =
    LazyLock::new(|| parking_lot::Mutex::new(HashMap::new()));

/// Trait for bank API adapters
#[async_trait]
pub trait BankAdapter: Send + Sync {
    /// Test connectivity to the bank API
    async fn test_connection(&self) -> Result<BankConnectionStatus, ApiError>;

    /// Initiate a payment (havale / EFT)
    async fn initiate_payment(
        &self,
        payment: PaymentInitiation,
    ) -> Result<PaymentInitiationResponse, ApiError>;

    /// Check the status of a previously initiated payment
    async fn check_payment_status(&self, tracking_id: &str) -> Result<PaymentStatus, ApiError>;

    /// Fetch CAMT.053 statements for an account in a date range
    async fn fetch_statements(
        &self,
        account_id: i64,
        from_date: NaiveDate,
        to_date: NaiveDate,
    ) -> Result<Vec<CamtStatement>, ApiError>;
}

/// Type alias for boxed adapter
pub type BoxBankAdapter = Arc<dyn BankAdapter>;

/// Factory for creating bank adapters
pub struct BankAdapterFactory;

impl BankAdapterFactory {
    /// Create a mock adapter for the given bank code
    pub fn create_mock(bank_code: BankCode, credentials: BankApiCredentials) -> BoxBankAdapter {
        match bank_code {
            BankCode::Halkbank => Arc::new(MockHalkbankAdapter::new(credentials)) as BoxBankAdapter,
            BankCode::Ziraat => Arc::new(MockZiraatAdapter::new(credentials)) as BoxBankAdapter,
            BankCode::IsBankasi => {
                Arc::new(MockIsBankasiAdapter::new(credentials)) as BoxBankAdapter
            }
            _ => Arc::new(MockGenericAdapter::new(credentials)) as BoxBankAdapter,
        }
    }
}

/// Shared state for mock adapters
struct MockAdapterState {
    credentials: BankApiCredentials,
    next_tracking_id: std::sync::atomic::AtomicU64,
}

impl MockAdapterState {
    fn new(credentials: BankApiCredentials) -> Self {
        Self {
            credentials,
            next_tracking_id: std::sync::atomic::AtomicU64::new(1),
        }
    }

    fn generate_tracking_id(&self, prefix: &str) -> String {
        let id = self
            .next_tracking_id
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        format!("{}-{}-{}", prefix, id, chrono::Utc::now().timestamp())
    }

    fn store_status(&self, tracking_id: String, status: PaymentStatus) {
        SHARED_PAYMENT_STATUSES.lock().insert(tracking_id, status);
    }

    fn get_status(&self, tracking_id: &str) -> Option<PaymentStatus> {
        SHARED_PAYMENT_STATUSES.lock().get(tracking_id).copied()
    }
}

/// Mock adapter for Halkbank
pub struct MockHalkbankAdapter {
    state: Arc<MockAdapterState>,
}

impl MockHalkbankAdapter {
    pub fn new(credentials: BankApiCredentials) -> Self {
        Self {
            state: Arc::new(MockAdapterState::new(credentials)),
        }
    }
}

#[async_trait]
impl BankAdapter for MockHalkbankAdapter {
    async fn test_connection(&self) -> Result<BankConnectionStatus, ApiError> {
        if self.state.credentials.api_key.is_empty() {
            return Ok(BankConnectionStatus::Error);
        }
        Ok(BankConnectionStatus::Connected)
    }

    async fn initiate_payment(
        &self,
        payment: PaymentInitiation,
    ) -> Result<PaymentInitiationResponse, ApiError> {
        if payment.amount <= Decimal::ZERO {
            return Err(ApiError::Validation(
                "Payment amount must be positive".to_string(),
            ));
        }

        let tracking_id = self.state.generate_tracking_id("HLK");
        let bank_ref = format!("HLK-REF-{}", Utc::now().timestamp());

        self.state
            .store_status(tracking_id.clone(), PaymentStatus::Pending);

        Ok(PaymentInitiationResponse {
            tracking_id,
            status: PaymentStatus::Pending,
            bank_reference: Some(bank_ref),
            created_at: Utc::now(),
        })
    }

    async fn check_payment_status(&self, tracking_id: &str) -> Result<PaymentStatus, ApiError> {
        self.state
            .get_status(tracking_id)
            .ok_or_else(|| ApiError::NotFound(format!("Payment {} not found", tracking_id)))
    }

    async fn fetch_statements(
        &self,
        _account_id: i64,
        from_date: NaiveDate,
        to_date: NaiveDate,
    ) -> Result<Vec<CamtStatement>, ApiError> {
        if from_date > to_date {
            return Err(ApiError::BadRequest(
                "from_date must be before to_date".to_string(),
            ));
        }

        let entry_count = (to_date - from_date).num_days().max(1) as usize;
        let mut entries = Vec::with_capacity(entry_count);

        for i in 0..entry_count.min(10) {
            let date = from_date + chrono::Duration::days(i as i64);
            entries.push(crate::domain::bank::model::CamtEntry {
                entry_date: date,
                amount: Decimal::new(1000 + i as i64 * 100, 2),
                currency: "TRY".to_string(),
                credit_debit: if i % 2 == 0 {
                    "CRDT".to_string()
                } else {
                    "DBIT".to_string()
                },
                reference: Some(format!("HLK-REF-{}", i)),
                description: Some(format!("Mock Halkbank transaction {}", i)),
                counterparty_name: Some("Test Counterparty".to_string()),
                counterparty_iban: Some("TR001234567890123456789012".to_string()),
            });
        }

        Ok(vec![CamtStatement {
            statement_id: format!("HLK-STMT-{}", Utc::now().timestamp()),
            creation_date: Utc::now(),
            account_iban: "TR001234567890123456789012".to_string(),
            entries,
        }])
    }
}

/// Mock adapter for Ziraat Bankasi
pub struct MockZiraatAdapter {
    state: Arc<MockAdapterState>,
}

impl MockZiraatAdapter {
    pub fn new(credentials: BankApiCredentials) -> Self {
        Self {
            state: Arc::new(MockAdapterState::new(credentials)),
        }
    }
}

#[async_trait]
impl BankAdapter for MockZiraatAdapter {
    async fn test_connection(&self) -> Result<BankConnectionStatus, ApiError> {
        if self.state.credentials.api_secret.len() < 8 {
            return Ok(BankConnectionStatus::Error);
        }
        Ok(BankConnectionStatus::Connected)
    }

    async fn initiate_payment(
        &self,
        payment: PaymentInitiation,
    ) -> Result<PaymentInitiationResponse, ApiError> {
        if payment.destination_iban.is_none() && payment.destination_account_no.is_none() {
            return Err(ApiError::Validation(
                "Destination IBAN or account number is required".to_string(),
            ));
        }

        let tracking_id = self.state.generate_tracking_id("ZRT");
        let bank_ref = format!("ZRT-REF-{}", Utc::now().timestamp());

        self.state
            .store_status(tracking_id.clone(), PaymentStatus::Processing);

        Ok(PaymentInitiationResponse {
            tracking_id,
            status: PaymentStatus::Processing,
            bank_reference: Some(bank_ref),
            created_at: Utc::now(),
        })
    }

    async fn check_payment_status(&self, tracking_id: &str) -> Result<PaymentStatus, ApiError> {
        self.state
            .get_status(tracking_id)
            .ok_or_else(|| ApiError::NotFound(format!("Payment {} not found", tracking_id)))
    }

    async fn fetch_statements(
        &self,
        _account_id: i64,
        from_date: NaiveDate,
        to_date: NaiveDate,
    ) -> Result<Vec<CamtStatement>, ApiError> {
        if from_date > to_date {
            return Err(ApiError::BadRequest(
                "from_date must be before to_date".to_string(),
            ));
        }

        let entry_count = (to_date - from_date).num_days().max(1) as usize;
        let mut entries = Vec::with_capacity(entry_count);

        for i in 0..entry_count.min(10) {
            let date = from_date + chrono::Duration::days(i as i64);
            entries.push(crate::domain::bank::model::CamtEntry {
                entry_date: date,
                amount: Decimal::new(2000 + i as i64 * 200, 2),
                currency: "TRY".to_string(),
                credit_debit: if i % 2 == 0 {
                    "DBIT".to_string()
                } else {
                    "CRDT".to_string()
                },
                reference: Some(format!("ZRT-REF-{}", i)),
                description: Some(format!("Mock Ziraat transaction {}", i)),
                counterparty_name: Some("Test Counterparty".to_string()),
                counterparty_iban: Some("TR009876543210987654321098".to_string()),
            });
        }

        Ok(vec![CamtStatement {
            statement_id: format!("ZRT-STMT-{}", Utc::now().timestamp()),
            creation_date: Utc::now(),
            account_iban: "TR009876543210987654321098".to_string(),
            entries,
        }])
    }
}

/// Mock adapter for Is Bankasi
pub struct MockIsBankasiAdapter {
    state: Arc<MockAdapterState>,
}

impl MockIsBankasiAdapter {
    pub fn new(credentials: BankApiCredentials) -> Self {
        Self {
            state: Arc::new(MockAdapterState::new(credentials)),
        }
    }
}

#[async_trait]
impl BankAdapter for MockIsBankasiAdapter {
    async fn test_connection(&self) -> Result<BankConnectionStatus, ApiError> {
        if self.state.credentials.base_url.is_empty() {
            return Ok(BankConnectionStatus::Error);
        }
        Ok(BankConnectionStatus::Connected)
    }

    async fn initiate_payment(
        &self,
        payment: PaymentInitiation,
    ) -> Result<PaymentInitiationResponse, ApiError> {
        if payment.beneficiary_name.len() < 2 {
            return Err(ApiError::Validation(
                "Beneficiary name is too short".to_string(),
            ));
        }

        let tracking_id = self.state.generate_tracking_id("ISB");
        let bank_ref = format!("ISB-REF-{}", Utc::now().timestamp());

        self.state
            .store_status(tracking_id.clone(), PaymentStatus::Pending);

        Ok(PaymentInitiationResponse {
            tracking_id,
            status: PaymentStatus::Pending,
            bank_reference: Some(bank_ref),
            created_at: Utc::now(),
        })
    }

    async fn check_payment_status(&self, tracking_id: &str) -> Result<PaymentStatus, ApiError> {
        self.state
            .get_status(tracking_id)
            .ok_or_else(|| ApiError::NotFound(format!("Payment {} not found", tracking_id)))
    }

    async fn fetch_statements(
        &self,
        _account_id: i64,
        from_date: NaiveDate,
        to_date: NaiveDate,
    ) -> Result<Vec<CamtStatement>, ApiError> {
        if from_date > to_date {
            return Err(ApiError::BadRequest(
                "from_date must be before to_date".to_string(),
            ));
        }

        let entry_count = (to_date - from_date).num_days().max(1) as usize;
        let mut entries = Vec::with_capacity(entry_count);

        for i in 0..entry_count.min(10) {
            let date = from_date + chrono::Duration::days(i as i64);
            entries.push(crate::domain::bank::model::CamtEntry {
                entry_date: date,
                amount: Decimal::new(3000 + i as i64 * 300, 2),
                currency: "TRY".to_string(),
                credit_debit: if i % 2 == 0 {
                    "CRDT".to_string()
                } else {
                    "DBIT".to_string()
                },
                reference: Some(format!("ISB-REF-{}", i)),
                description: Some(format!("Mock Is Bankasi transaction {}", i)),
                counterparty_name: Some("Test Counterparty".to_string()),
                counterparty_iban: Some("TR005678901234567890123456".to_string()),
            });
        }

        Ok(vec![CamtStatement {
            statement_id: format!("ISB-STMT-{}", Utc::now().timestamp()),
            creation_date: Utc::now(),
            account_iban: "TR005678901234567890123456".to_string(),
            entries,
        }])
    }
}

/// Generic mock adapter for unsupported banks
pub struct MockGenericAdapter {
    state: Arc<MockAdapterState>,
}

impl MockGenericAdapter {
    pub fn new(credentials: BankApiCredentials) -> Self {
        Self {
            state: Arc::new(MockAdapterState::new(credentials)),
        }
    }
}

#[async_trait]
impl BankAdapter for MockGenericAdapter {
    async fn test_connection(&self) -> Result<BankConnectionStatus, ApiError> {
        Ok(BankConnectionStatus::Connected)
    }

    async fn initiate_payment(
        &self,
        _payment: PaymentInitiation,
    ) -> Result<PaymentInitiationResponse, ApiError> {
        let tracking_id = self.state.generate_tracking_id("GEN");
        self.state
            .store_status(tracking_id.clone(), PaymentStatus::Pending);

        Ok(PaymentInitiationResponse {
            tracking_id,
            status: PaymentStatus::Pending,
            bank_reference: None,
            created_at: Utc::now(),
        })
    }

    async fn check_payment_status(&self, tracking_id: &str) -> Result<PaymentStatus, ApiError> {
        self.state
            .get_status(tracking_id)
            .ok_or_else(|| ApiError::NotFound(format!("Payment {} not found", tracking_id)))
    }

    async fn fetch_statements(
        &self,
        _account_id: i64,
        _from_date: NaiveDate,
        _to_date: NaiveDate,
    ) -> Result<Vec<CamtStatement>, ApiError> {
        Ok(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::bank::model::BankCode;
    use rust_decimal_macros::dec;

    fn test_credentials(bank_code: BankCode) -> BankApiCredentials {
        BankApiCredentials {
            bank_code,
            api_key: "test-api-key".to_string(),
            api_secret: "test-api-secret-123".to_string(),
            base_url: "https://api.example.com".to_string(),
            client_id: Some("test-client".to_string()),
        }
    }

    #[tokio::test]
    async fn test_halkbank_connection_success() {
        let adapter = MockHalkbankAdapter::new(test_credentials(BankCode::Halkbank));
        let status = adapter.test_connection().await.unwrap();
        assert_eq!(status, BankConnectionStatus::Connected);
    }

    #[tokio::test]
    async fn test_halkbank_connection_failure_empty_key() {
        let mut creds = test_credentials(BankCode::Halkbank);
        creds.api_key = "".to_string();
        let adapter = MockHalkbankAdapter::new(creds);
        let status = adapter.test_connection().await.unwrap();
        assert_eq!(status, BankConnectionStatus::Error);
    }

    #[tokio::test]
    async fn test_halkbank_initiate_payment() {
        let adapter = MockHalkbankAdapter::new(test_credentials(BankCode::Halkbank));
        let payment = PaymentInitiation {
            source_account_id: 1,
            destination_iban: Some("TR000123456789012345678901".to_string()),
            destination_account_no: None,
            beneficiary_name: "Test Recipient".to_string(),
            amount: dec!(1000.00),
            currency: "TRY".to_string(),
            description: None,
            payment_type: crate::domain::bank::model::PaymentType::Havale,
            tenant_id: 1,
        };

        let response = adapter.initiate_payment(payment).await.unwrap();
        assert!(response.tracking_id.starts_with("HLK-"));
        assert_eq!(response.status, PaymentStatus::Pending);
        assert!(response.bank_reference.is_some());
    }

    #[tokio::test]
    async fn test_halkbank_payment_zero_amount_fails() {
        let adapter = MockHalkbankAdapter::new(test_credentials(BankCode::Halkbank));
        let payment = PaymentInitiation {
            source_account_id: 1,
            destination_iban: Some("TR000123456789012345678901".to_string()),
            destination_account_no: None,
            beneficiary_name: "Test".to_string(),
            amount: Decimal::ZERO,
            currency: "TRY".to_string(),
            description: None,
            payment_type: crate::domain::bank::model::PaymentType::Havale,
            tenant_id: 1,
        };

        let result = adapter.initiate_payment(payment).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_ziraat_connection_success() {
        let adapter = MockZiraatAdapter::new(test_credentials(BankCode::Ziraat));
        let status = adapter.test_connection().await.unwrap();
        assert_eq!(status, BankConnectionStatus::Connected);
    }

    #[tokio::test]
    async fn test_ziraat_connection_failure_short_secret() {
        let mut creds = test_credentials(BankCode::Ziraat);
        creds.api_secret = "short".to_string();
        let adapter = MockZiraatAdapter::new(creds);
        let status = adapter.test_connection().await.unwrap();
        assert_eq!(status, BankConnectionStatus::Error);
    }

    #[tokio::test]
    async fn test_ziraat_payment_requires_destination() {
        let adapter = MockZiraatAdapter::new(test_credentials(BankCode::Ziraat));
        let payment = PaymentInitiation {
            source_account_id: 1,
            destination_iban: None,
            destination_account_no: None,
            beneficiary_name: "Test".to_string(),
            amount: dec!(500.00),
            currency: "TRY".to_string(),
            description: None,
            payment_type: crate::domain::bank::model::PaymentType::Eft,
            tenant_id: 1,
        };

        let result = adapter.initiate_payment(payment).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_is_bankasi_connection_success() {
        let adapter = MockIsBankasiAdapter::new(test_credentials(BankCode::IsBankasi));
        let status = adapter.test_connection().await.unwrap();
        assert_eq!(status, BankConnectionStatus::Connected);
    }

    #[tokio::test]
    async fn test_is_bankasi_payment_short_name_fails() {
        let adapter = MockIsBankasiAdapter::new(test_credentials(BankCode::IsBankasi));
        let payment = PaymentInitiation {
            source_account_id: 1,
            destination_iban: Some("TR000123456789012345678901".to_string()),
            destination_account_no: None,
            beneficiary_name: "A".to_string(),
            amount: dec!(100.00),
            currency: "TRY".to_string(),
            description: None,
            payment_type: crate::domain::bank::model::PaymentType::Havale,
            tenant_id: 1,
        };

        let result = adapter.initiate_payment(payment).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_payment_status_tracking() {
        let adapter = MockHalkbankAdapter::new(test_credentials(BankCode::Halkbank));
        let payment = PaymentInitiation {
            source_account_id: 1,
            destination_iban: Some("TR000123456789012345678901".to_string()),
            destination_account_no: None,
            beneficiary_name: "Test".to_string(),
            amount: dec!(100.00),
            currency: "TRY".to_string(),
            description: None,
            payment_type: crate::domain::bank::model::PaymentType::Havale,
            tenant_id: 1,
        };

        let response = adapter.initiate_payment(payment).await.unwrap();
        let status = adapter
            .check_payment_status(&response.tracking_id)
            .await
            .unwrap();
        assert_eq!(status, PaymentStatus::Pending);
    }

    #[tokio::test]
    async fn test_fetch_statements_invalid_range() {
        let adapter = MockHalkbankAdapter::new(test_credentials(BankCode::Halkbank));
        let from = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        let to = NaiveDate::from_ymd_opt(2024, 1, 10).unwrap();

        let result = adapter.fetch_statements(1, from, to).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_fetch_statements_success() {
        let adapter = MockZiraatAdapter::new(test_credentials(BankCode::Ziraat));
        let from = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let to = NaiveDate::from_ymd_opt(2024, 1, 5).unwrap();

        let statements = adapter.fetch_statements(1, from, to).await.unwrap();
        assert_eq!(statements.len(), 1);
        assert!(!statements[0].entries.is_empty());
    }

    #[tokio::test]
    async fn test_factory_creates_correct_adapter() {
        let creds = test_credentials(BankCode::Halkbank);
        let adapter = BankAdapterFactory::create_mock(BankCode::Halkbank, creds);
        let status = adapter.test_connection().await.unwrap();
        assert_eq!(status, BankConnectionStatus::Connected);
    }
}
