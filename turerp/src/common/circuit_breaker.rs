//! Circuit breaker pattern for external service resilience
//!
//! Provides per-service circuit breakers (GIB, email, SMS, bank, webhook)
//! with Closed/Open/HalfOpen state transitions and Prometheus metrics.

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use utoipa::ToSchema;

use crate::error::ApiError;

// ---------------------------------------------------------------------------
// Circuit breaker state
// ---------------------------------------------------------------------------

/// Circuit breaker state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum CircuitState {
    /// Normal operation, requests pass through
    Closed,
    /// Failing fast, rejecting requests
    Open,
    /// Testing if service has recovered
    HalfOpen,
}

impl std::fmt::Display for CircuitState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Closed => write!(f, "closed"),
            Self::Open => write!(f, "open"),
            Self::HalfOpen => write!(f, "half_open"),
        }
    }
}

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Circuit breaker configuration
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CircuitBreakerConfig {
    /// Number of consecutive failures before opening the circuit
    pub failure_threshold: u32,
    /// Number of consecutive successes in half-open before closing
    pub success_threshold: u32,
    /// Duration the circuit stays open before transitioning to half-open
    pub timeout_duration_ms: u64,
    /// Max calls allowed in half-open state
    pub half_open_max_calls: u32,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 3,
            timeout_duration_ms: 30_000,
            half_open_max_calls: 3,
        }
    }
}

impl CircuitBreakerConfig {
    /// Configuration optimized for GIB gateway (slower, government service)
    pub fn gib_default() -> Self {
        Self {
            failure_threshold: 3,
            success_threshold: 2,
            timeout_duration_ms: 60_000,
            half_open_max_calls: 2,
        }
    }

    /// Configuration optimized for email providers
    pub fn email_default() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 3,
            timeout_duration_ms: 30_000,
            half_open_max_calls: 3,
        }
    }

    /// Configuration optimized for SMS providers
    pub fn sms_default() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 3,
            timeout_duration_ms: 20_000,
            half_open_max_calls: 3,
        }
    }

    /// Configuration optimized for bank integrations
    pub fn bank_default() -> Self {
        Self {
            failure_threshold: 3,
            success_threshold: 2,
            timeout_duration_ms: 45_000,
            half_open_max_calls: 2,
        }
    }

    /// Configuration optimized for webhook delivery
    pub fn webhook_default() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 2,
            timeout_duration_ms: 15_000,
            half_open_max_calls: 3,
        }
    }

    /// Configuration optimized for file storage (S3/local)
    pub fn file_storage_default() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 2,
            timeout_duration_ms: 30_000,
            half_open_max_calls: 3,
        }
    }
}

// ---------------------------------------------------------------------------
// Inner state (protected by Mutex)
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct Inner {
    state: CircuitState,
    failure_count: u32,
    success_count: u32,
    half_open_calls: u32,
    last_failure_time: Option<Instant>,
    opened_at: Option<Instant>,
    config: CircuitBreakerConfig,
    total_failures: u64,
    total_successes: u64,
    state_changes: u64,
}

// ---------------------------------------------------------------------------
// Circuit breaker
// ---------------------------------------------------------------------------

/// Thread-safe circuit breaker for a single service
#[derive(Debug)]
pub struct CircuitBreaker {
    inner: Mutex<Inner>,
    service_name: String,
}

impl CircuitBreaker {
    /// Create a new circuit breaker for a named service
    pub fn new(service_name: impl Into<String>, config: CircuitBreakerConfig) -> Self {
        let name = service_name.into();
        Self {
            inner: Mutex::new(Inner {
                state: CircuitState::Closed,
                failure_count: 0,
                success_count: 0,
                half_open_calls: 0,
                last_failure_time: None,
                opened_at: None,
                config,
                total_failures: 0,
                total_successes: 0,
                state_changes: 0,
            }),
            service_name: name,
        }
    }

    /// Execute a fallible operation through the circuit breaker
    pub async fn call<T, F, Fut>(&self, operation: F) -> Result<T, ApiError>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T, ApiError>>,
    {
        {
            let mut inner = self.inner.lock();

            // Check if we need to transition Open -> HalfOpen
            if matches!(inner.state, CircuitState::Open) {
                let timeout = Duration::from_millis(inner.config.timeout_duration_ms);
                if let Some(opened_at) = inner.opened_at {
                    if opened_at.elapsed() >= timeout {
                        tracing::info!(
                            "Circuit breaker '{}' transitioning Open -> HalfOpen",
                            self.service_name
                        );
                        inner.state = CircuitState::HalfOpen;
                        inner.half_open_calls = 0;
                        inner.success_count = 0;
                        inner.state_changes += 1;
                        metrics::counter!(
                            "circuit_breaker_state_changes_total",
                            "service" => self.service_name.clone(),
                            "to_state" => "half_open"
                        )
                        .increment(1);
                    } else {
                        metrics::counter!(
                            "circuit_breaker_rejections_total",
                            "service" => self.service_name.clone()
                        )
                        .increment(1);
                        return Err(ApiError::ServiceUnavailable(format!(
                            "Service '{}' is unavailable (circuit open)",
                            self.service_name
                        )));
                    }
                }
            }

            // Check HalfOpen call limit
            if matches!(inner.state, CircuitState::HalfOpen) {
                if inner.half_open_calls >= inner.config.half_open_max_calls {
                    metrics::counter!(
                        "circuit_breaker_rejections_total",
                        "service" => self.service_name.clone()
                    )
                    .increment(1);
                    return Err(ApiError::ServiceUnavailable(format!(
                        "Service '{}' is unavailable (circuit half-open, max calls reached)",
                        self.service_name
                    )));
                }
                inner.half_open_calls += 1;
            }
        }

        // Execute the operation
        let result = operation().await;

        match &result {
            Ok(_) => self.record_success(),
            Err(_) => self.record_failure(),
        }

        result
    }

    /// Record a successful call (used for manual reporting or async completion)
    pub fn record_success(&self) {
        let mut inner = self.inner.lock();
        inner.total_successes += 1;
        inner.last_failure_time = None;
        inner.failure_count = 0;

        match inner.state {
            CircuitState::HalfOpen => {
                inner.success_count += 1;
                if inner.success_count >= inner.config.success_threshold {
                    tracing::info!(
                        "Circuit breaker '{}' transitioning HalfOpen -> Closed",
                        self.service_name
                    );
                    inner.state = CircuitState::Closed;
                    inner.half_open_calls = 0;
                    inner.success_count = 0;
                    inner.state_changes += 1;
                    metrics::counter!(
                        "circuit_breaker_state_changes_total",
                        "service" => self.service_name.clone(),
                        "to_state" => "closed"
                    )
                    .increment(1);
                }
            }
            CircuitState::Closed => {
                // Reset success count tracking in closed state
                inner.success_count = 0;
            }
            CircuitState::Open => {
                // Should not happen, but handle gracefully
            }
        }
    }

    /// Record a failed call
    pub fn record_failure(&self) {
        let mut inner = self.inner.lock();
        inner.total_failures += 1;
        inner.failure_count += 1;
        inner.last_failure_time = Some(Instant::now());

        match inner.state {
            CircuitState::Closed => {
                if inner.failure_count >= inner.config.failure_threshold {
                    tracing::warn!(
                        "Circuit breaker '{}' transitioning Closed -> Open ({} consecutive failures)",
                        self.service_name,
                        inner.failure_count
                    );
                    inner.state = CircuitState::Open;
                    inner.opened_at = Some(Instant::now());
                    inner.state_changes += 1;
                    metrics::counter!(
                        "circuit_breaker_state_changes_total",
                        "service" => self.service_name.clone(),
                        "to_state" => "open"
                    )
                    .increment(1);
                    metrics::counter!(
                        "circuit_breaker_opens_total",
                        "service" => self.service_name.clone()
                    )
                    .increment(1);
                }
            }
            CircuitState::HalfOpen => {
                tracing::warn!(
                    "Circuit breaker '{}' transitioning HalfOpen -> Open (failure in half-open)",
                    self.service_name
                );
                inner.state = CircuitState::Open;
                inner.opened_at = Some(Instant::now());
                inner.half_open_calls = 0;
                inner.success_count = 0;
                inner.state_changes += 1;
                metrics::counter!(
                    "circuit_breaker_state_changes_total",
                    "service" => self.service_name.clone(),
                    "to_state" => "open"
                )
                .increment(1);
                metrics::counter!(
                    "circuit_breaker_opens_total",
                    "service" => self.service_name.clone()
                )
                .increment(1);
            }
            CircuitState::Open => {
                // Already open, just update last failure time
            }
        }
    }

    /// Get current state
    pub fn get_state(&self) -> CircuitState {
        let mut inner = self.inner.lock();

        // Auto-transition check for Open -> HalfOpen on read
        if matches!(inner.state, CircuitState::Open) {
            let timeout = Duration::from_millis(inner.config.timeout_duration_ms);
            if let Some(opened_at) = inner.opened_at {
                if opened_at.elapsed() >= timeout {
                    inner.state = CircuitState::HalfOpen;
                    inner.half_open_calls = 0;
                    inner.success_count = 0;
                    inner.state_changes += 1;
                    metrics::counter!(
                        "circuit_breaker_state_changes_total",
                        "service" => self.service_name.clone(),
                        "to_state" => "half_open"
                    )
                    .increment(1);
                }
            }
        }

        inner.state
    }

    /// Reset the circuit breaker to Closed state
    pub fn reset(&self) {
        let mut inner = self.inner.lock();
        let old_state = inner.state;
        inner.state = CircuitState::Closed;
        inner.failure_count = 0;
        inner.success_count = 0;
        inner.half_open_calls = 0;
        inner.last_failure_time = None;
        inner.opened_at = None;

        if old_state != CircuitState::Closed {
            inner.state_changes += 1;
            metrics::counter!(
                "circuit_breaker_state_changes_total",
                "service" => self.service_name.clone(),
                "to_state" => "closed"
            )
            .increment(1);
        }

        tracing::info!(
            "Circuit breaker '{}' manually reset to Closed",
            self.service_name
        );
    }

    /// Get service name
    pub fn service_name(&self) -> &str {
        &self.service_name
    }

    /// Get snapshot of current statistics
    pub fn stats(&self) -> CircuitBreakerStats {
        let inner = self.inner.lock();
        CircuitBreakerStats {
            service_name: self.service_name.clone(),
            state: inner.state,
            failure_count: inner.failure_count,
            success_count: inner.success_count,
            half_open_calls: inner.half_open_calls,
            total_failures: inner.total_failures,
            total_successes: inner.total_successes,
            state_changes: inner.state_changes,
            last_failure_time_secs: inner.last_failure_time.map(|t| t.elapsed().as_secs()),
            opened_at_secs: inner.opened_at.map(|t| t.elapsed().as_secs()),
        }
    }
}

// ---------------------------------------------------------------------------
// Stats
// ---------------------------------------------------------------------------

/// Circuit breaker statistics snapshot
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CircuitBreakerStats {
    pub service_name: String,
    pub state: CircuitState,
    pub failure_count: u32,
    pub success_count: u32,
    pub half_open_calls: u32,
    pub total_failures: u64,
    pub total_successes: u64,
    pub state_changes: u64,
    pub last_failure_time_secs: Option<u64>,
    pub opened_at_secs: Option<u64>,
}

// ---------------------------------------------------------------------------
// Registry
// ---------------------------------------------------------------------------

/// Known service names for circuit breakers
pub const SERVICE_GIB: &str = "gib";
pub const SERVICE_EMAIL: &str = "email";
pub const SERVICE_SMS: &str = "sms";
pub const SERVICE_BANK: &str = "bank";
pub const SERVICE_WEBHOOK: &str = "webhook";
pub const SERVICE_FILE_STORAGE: &str = "file_storage";

/// Registry holding circuit breakers for all external services
#[derive(Debug)]
pub struct CircuitBreakerRegistry {
    breakers: Mutex<HashMap<String, Arc<CircuitBreaker>>>,
}

impl CircuitBreakerRegistry {
    /// Create a new registry with default circuit breakers for all known services
    pub fn new() -> Self {
        let mut breakers = HashMap::new();
        breakers.insert(
            SERVICE_GIB.to_string(),
            Arc::new(CircuitBreaker::new(
                SERVICE_GIB,
                CircuitBreakerConfig::gib_default(),
            )),
        );
        breakers.insert(
            SERVICE_EMAIL.to_string(),
            Arc::new(CircuitBreaker::new(
                SERVICE_EMAIL,
                CircuitBreakerConfig::email_default(),
            )),
        );
        breakers.insert(
            SERVICE_SMS.to_string(),
            Arc::new(CircuitBreaker::new(
                SERVICE_SMS,
                CircuitBreakerConfig::sms_default(),
            )),
        );
        breakers.insert(
            SERVICE_BANK.to_string(),
            Arc::new(CircuitBreaker::new(
                SERVICE_BANK,
                CircuitBreakerConfig::bank_default(),
            )),
        );
        breakers.insert(
            SERVICE_WEBHOOK.to_string(),
            Arc::new(CircuitBreaker::new(
                SERVICE_WEBHOOK,
                CircuitBreakerConfig::webhook_default(),
            )),
        );
        breakers.insert(
            SERVICE_FILE_STORAGE.to_string(),
            Arc::new(CircuitBreaker::new(
                SERVICE_FILE_STORAGE,
                CircuitBreakerConfig::file_storage_default(),
            )),
        );

        Self {
            breakers: Mutex::new(breakers),
        }
    }

    /// Get a circuit breaker by service name
    pub fn get(&self, service: &str) -> Option<Arc<CircuitBreaker>> {
        self.breakers.lock().get(service).cloned()
    }

    /// Get all circuit breaker statistics
    pub fn list_all(&self) -> Vec<CircuitBreakerStats> {
        self.breakers.lock().values().map(|cb| cb.stats()).collect()
    }

    /// Reset a specific circuit breaker
    pub fn reset(&self, service: &str) -> bool {
        if let Some(cb) = self.breakers.lock().get(service) {
            cb.reset();
            true
        } else {
            false
        }
    }

    /// Register a custom circuit breaker
    pub fn register(&self, service: impl Into<String>, breaker: Arc<CircuitBreaker>) {
        self.breakers.lock().insert(service.into(), breaker);
    }
}

impl Default for CircuitBreakerRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> CircuitBreakerConfig {
        CircuitBreakerConfig {
            failure_threshold: 3,
            success_threshold: 2,
            timeout_duration_ms: 100,
            half_open_max_calls: 2,
        }
    }

    #[test]
    fn test_new_circuit_breaker_closed() {
        let cb = CircuitBreaker::new("test", test_config());
        assert!(matches!(cb.get_state(), CircuitState::Closed));
    }

    #[tokio::test]
    async fn test_call_success() {
        let cb = CircuitBreaker::new("test", test_config());
        let result = cb.call(|| async { Ok::<_, ApiError>(42) }).await;
        assert_eq!(result.unwrap(), 42);
        assert!(matches!(cb.get_state(), CircuitState::Closed));
    }

    #[tokio::test]
    async fn test_call_failure_then_open() {
        let cb = CircuitBreaker::new("test", test_config());

        // 3 failures should open the circuit
        for _ in 0..3 {
            let _ = cb
                .call(|| async { Err::<i32, _>(ApiError::Internal("fail".to_string())) })
                .await;
        }

        assert!(matches!(cb.get_state(), CircuitState::Open));

        // Next call should fail fast
        let result = cb.call(|| async { Ok::<_, ApiError>(42) }).await;
        assert!(matches!(result, Err(ApiError::ServiceUnavailable(_))));
    }

    #[tokio::test]
    async fn test_half_open_then_close() {
        let cb = CircuitBreaker::new("test", test_config());

        // Open the circuit
        for _ in 0..3 {
            let _ = cb
                .call(|| async { Err::<i32, _>(ApiError::Internal("fail".to_string())) })
                .await;
        }
        assert!(matches!(cb.get_state(), CircuitState::Open));

        // Wait for timeout
        tokio::time::sleep(Duration::from_millis(150)).await;

        // Now it should be HalfOpen
        assert!(matches!(cb.get_state(), CircuitState::HalfOpen));

        // 2 successes should close it
        for _ in 0..2 {
            let _ = cb.call(|| async { Ok::<_, ApiError>(42) }).await;
        }

        assert!(matches!(cb.get_state(), CircuitState::Closed));
    }

    #[tokio::test]
    async fn test_half_open_failure_reopens() {
        let cb = CircuitBreaker::new("test", test_config());

        // Open the circuit
        for _ in 0..3 {
            let _ = cb
                .call(|| async { Err::<i32, _>(ApiError::Internal("fail".to_string())) })
                .await;
        }

        // Wait for timeout
        tokio::time::sleep(Duration::from_millis(150)).await;

        // Single failure in half-open should reopen
        let _ = cb
            .call(|| async { Err::<i32, _>(ApiError::Internal("fail".to_string())) })
            .await;

        assert!(matches!(cb.get_state(), CircuitState::Open));
    }

    #[test]
    fn test_manual_reset() {
        let cb = CircuitBreaker::new("test", test_config());
        cb.record_failure();
        cb.record_failure();
        cb.record_failure();
        assert!(matches!(cb.get_state(), CircuitState::Open));

        cb.reset();
        assert!(matches!(cb.get_state(), CircuitState::Closed));
    }

    #[test]
    fn test_registry() {
        let registry = CircuitBreakerRegistry::new();
        let all = registry.list_all();
        assert_eq!(all.len(), 6);

        assert!(registry.get(SERVICE_GIB).is_some());
        assert!(registry.get(SERVICE_EMAIL).is_some());
        assert!(registry.get(SERVICE_SMS).is_some());
        assert!(registry.get(SERVICE_BANK).is_some());
        assert!(registry.get(SERVICE_WEBHOOK).is_some());
        assert!(registry.get(SERVICE_FILE_STORAGE).is_some());
    }

    #[test]
    fn test_registry_reset() {
        let registry = CircuitBreakerRegistry::new();
        let cb = registry.get(SERVICE_GIB).unwrap();
        cb.record_failure();
        cb.record_failure();
        cb.record_failure();
        assert!(matches!(cb.get_state(), CircuitState::Open));

        assert!(registry.reset(SERVICE_GIB));
        assert!(matches!(cb.get_state(), CircuitState::Closed));

        assert!(!registry.reset("nonexistent"));
    }
}
