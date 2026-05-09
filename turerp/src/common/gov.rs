//! Government gateway integration module
//!
//! Provides gateway traits for GIB (Gelir İdaresi Başkanlığı) integration,
//! enabling e-Fatura document exchange with the Turkish tax authority.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::Arc;

use crate::common::circuit_breaker::{CircuitBreaker, SERVICE_GIB};
use crate::common::retry::RetryPolicy;
use crate::domain::efatura::model::EFaturaProfile;
use crate::error::ApiError;

/// Result of sending an invoice to GIB
#[derive(Debug, Clone)]
pub struct GibSendResult {
    pub success: bool,
    pub message: Option<String>,
    pub envelope_uuid: Option<String>,
}

/// Result of checking status from GIB
#[derive(Debug, Clone)]
pub struct GibStatusResult {
    pub status: String,
    pub response_code: Option<String>,
    pub response_desc: Option<String>,
}

/// Gateway trait for GIB (Gelir İdaresi Başkanlığı) integration
#[async_trait]
pub trait GibGateway: Send + Sync {
    /// Send an invoice XML to GIB
    async fn send_invoice(
        &self,
        xml: &str,
        profile: EFaturaProfile,
    ) -> Result<GibSendResult, ApiError>;

    /// Check the status of a previously sent invoice
    async fn check_status(&self, uuid: &str) -> Result<GibStatusResult, ApiError>;

    /// Retrieve incoming invoices since a given timestamp
    async fn get_incoming(&self, since: DateTime<Utc>) -> Result<Vec<String>, ApiError>;

    /// Cancel a previously sent invoice
    async fn cancel(&self, uuid: &str, reason: &str) -> Result<(), ApiError>;
}

/// Type alias for boxed GibGateway
pub type BoxGibGateway = std::sync::Arc<dyn GibGateway>;

// ---------------------------------------------------------------------------
// InMemoryGibGateway
// ---------------------------------------------------------------------------

struct Inner {
    sent: HashMap<String, String>, // uuid -> xml
}

/// In-memory GIB gateway for testing and development
pub struct InMemoryGibGateway {
    inner: Mutex<Inner>,
}

impl InMemoryGibGateway {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(Inner {
                sent: HashMap::new(),
            }),
        }
    }
}

impl Default for InMemoryGibGateway {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl GibGateway for InMemoryGibGateway {
    async fn send_invoice(
        &self,
        xml: &str,
        _profile: EFaturaProfile,
    ) -> Result<GibSendResult, ApiError> {
        let uuid = format!("envelope-{}", chrono::Utc::now().timestamp_millis());

        let mut inner = self.inner.lock();
        inner.sent.insert(uuid.clone(), xml.to_string());

        Ok(GibSendResult {
            success: true,
            message: Some("Invoice sent successfully (in-memory)".to_string()),
            envelope_uuid: Some(uuid),
        })
    }

    async fn check_status(&self, uuid: &str) -> Result<GibStatusResult, ApiError> {
        let inner = self.inner.lock();
        if inner.sent.contains_key(uuid) {
            Ok(GibStatusResult {
                status: "Accepted".to_string(),
                response_code: Some("200".to_string()),
                response_desc: Some("OK".to_string()),
            })
        } else {
            Err(ApiError::NotFound(format!(
                "Invoice with UUID {} not found",
                uuid
            )))
        }
    }

    async fn get_incoming(&self, _since: DateTime<Utc>) -> Result<Vec<String>, ApiError> {
        // In-memory gateway returns no incoming invoices
        Ok(vec![])
    }

    async fn cancel(&self, uuid: &str, _reason: &str) -> Result<(), ApiError> {
        let mut inner = self.inner.lock();
        if inner.sent.remove(uuid).is_some() {
            Ok(())
        } else {
            Err(ApiError::NotFound(format!(
                "Invoice with UUID {} not found",
                uuid
            )))
        }
    }
}

// ---------------------------------------------------------------------------
// ResilientGibGateway
// ---------------------------------------------------------------------------

/// GIB gateway wrapper with circuit breaker and retry protection
pub struct ResilientGibGateway {
    inner: BoxGibGateway,
    circuit_breaker: Arc<CircuitBreaker>,
    retry_policy: RetryPolicy,
}

impl ResilientGibGateway {
    /// Wrap an existing GIB gateway with resilience
    pub fn new(
        inner: BoxGibGateway,
        registry: &crate::common::circuit_breaker::CircuitBreakerRegistry,
    ) -> Self {
        let cb = registry.get(SERVICE_GIB).unwrap_or_else(|| {
            Arc::new(CircuitBreaker::new(
                SERVICE_GIB,
                crate::common::circuit_breaker::CircuitBreakerConfig::gib_default(),
            ))
        });
        Self {
            inner,
            circuit_breaker: cb,
            retry_policy: RetryPolicy::new(
                "gib_send",
                crate::common::retry::RetryConfig::gib_default(),
            ),
        }
    }
}

#[async_trait]
impl GibGateway for ResilientGibGateway {
    async fn send_invoice(
        &self,
        xml: &str,
        profile: EFaturaProfile,
    ) -> Result<GibSendResult, ApiError> {
        let inner = self.inner.clone();
        let xml = xml.to_string();
        self.circuit_breaker
            .call(|| async {
                let inner = inner.clone();
                let xml = xml.clone();
                self.retry_policy
                    .execute(move || {
                        let inner = inner.clone();
                        let xml = xml.clone();
                        {
                            let profile = profile.clone();
                            async move { inner.send_invoice(&xml, profile).await }
                        }
                    })
                    .await
            })
            .await
    }

    async fn check_status(&self, uuid: &str) -> Result<GibStatusResult, ApiError> {
        let inner = self.inner.clone();
        let uuid = uuid.to_string();
        self.circuit_breaker
            .call(|| async {
                let inner = inner.clone();
                let uuid = uuid.clone();
                self.retry_policy
                    .execute(move || {
                        let inner = inner.clone();
                        let uuid = uuid.clone();
                        async move { inner.check_status(&uuid).await }
                    })
                    .await
            })
            .await
    }

    async fn get_incoming(&self, since: DateTime<Utc>) -> Result<Vec<String>, ApiError> {
        let inner = self.inner.clone();
        self.circuit_breaker
            .call(|| async {
                let inner = inner.clone();
                self.retry_policy
                    .execute(move || {
                        let inner = inner.clone();
                        async move { inner.get_incoming(since).await }
                    })
                    .await
            })
            .await
    }

    async fn cancel(&self, uuid: &str, reason: &str) -> Result<(), ApiError> {
        let inner = self.inner.clone();
        let uuid = uuid.to_string();
        let reason = reason.to_string();
        self.circuit_breaker
            .call(|| async {
                let inner = inner.clone();
                let uuid = uuid.clone();
                let reason = reason.clone();
                self.retry_policy
                    .execute(move || {
                        let inner = inner.clone();
                        let uuid = uuid.clone();
                        let reason = reason.clone();
                        async move { inner.cancel(&uuid, &reason).await }
                    })
                    .await
            })
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_send_invoice() {
        let gateway = InMemoryGibGateway::new();

        let result = gateway
            .send_invoice("<Invoice/>", EFaturaProfile::TemelFatura)
            .await
            .unwrap();

        assert!(result.success);
        assert!(result.envelope_uuid.is_some());
        assert!(result.message.is_some());
    }

    #[tokio::test]
    async fn test_check_status() {
        let gateway = InMemoryGibGateway::new();

        let send_result = gateway
            .send_invoice("<Invoice/>", EFaturaProfile::TemelFatura)
            .await
            .unwrap();

        let uuid = send_result.envelope_uuid.unwrap();

        let status = gateway.check_status(&uuid).await.unwrap();
        assert_eq!(status.status, "Accepted");
        assert_eq!(status.response_code, Some("200".to_string()));

        // Unknown UUID
        let result = gateway.check_status("nonexistent").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_incoming() {
        let gateway = InMemoryGibGateway::new();

        let incoming = gateway.get_incoming(Utc::now()).await.unwrap();
        assert!(incoming.is_empty());
    }

    #[tokio::test]
    async fn test_cancel() {
        let gateway = InMemoryGibGateway::new();

        let send_result = gateway
            .send_invoice("<Invoice/>", EFaturaProfile::TemelFatura)
            .await
            .unwrap();

        let uuid = send_result.envelope_uuid.unwrap();

        gateway.cancel(&uuid, "Mistake").await.unwrap();

        // After cancel, status check should fail
        let result = gateway.check_status(&uuid).await;
        assert!(result.is_err());

        // Canceling unknown UUID
        let result = gateway.cancel("nonexistent", "reason").await;
        assert!(result.is_err());
    }
}
