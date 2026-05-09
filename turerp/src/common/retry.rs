//! Retry policy with exponential backoff and jitter
//!
//! Provides per-operation retry wrappers with configurable delays
//! and Prometheus-compatible metrics.

use rand::Rng;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use utoipa::ToSchema;

use crate::error::ApiError;

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Retry policy configuration
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    pub max_retries: u32,
    /// Base delay between retries in milliseconds
    pub base_delay_ms: u64,
    /// Maximum delay between retries in milliseconds
    pub max_delay_ms: u64,
    /// Jitter factor (0.0-1.0) to add randomness to delays
    pub jitter_factor: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            base_delay_ms: 500,
            max_delay_ms: 30_000,
            jitter_factor: 0.3,
        }
    }
}

impl RetryConfig {
    /// Configuration for GIB gateway calls
    pub fn gib_default() -> Self {
        Self {
            max_retries: 5,
            base_delay_ms: 1_000,
            max_delay_ms: 60_000,
            jitter_factor: 0.2,
        }
    }

    /// Configuration for email delivery
    pub fn email_default() -> Self {
        Self {
            max_retries: 3,
            base_delay_ms: 2_000,
            max_delay_ms: 30_000,
            jitter_factor: 0.3,
        }
    }

    /// Configuration for SMS delivery
    pub fn sms_default() -> Self {
        Self {
            max_retries: 3,
            base_delay_ms: 1_500,
            max_delay_ms: 20_000,
            jitter_factor: 0.3,
        }
    }

    /// Configuration for bank integrations
    pub fn bank_default() -> Self {
        Self {
            max_retries: 3,
            base_delay_ms: 2_000,
            max_delay_ms: 45_000,
            jitter_factor: 0.2,
        }
    }

    /// Configuration for webhook delivery
    pub fn webhook_default() -> Self {
        Self {
            max_retries: 5,
            base_delay_ms: 1_000,
            max_delay_ms: 30_000,
            jitter_factor: 0.4,
        }
    }

    /// Configuration for file storage operations
    pub fn file_storage_default() -> Self {
        Self {
            max_retries: 3,
            base_delay_ms: 500,
            max_delay_ms: 15_000,
            jitter_factor: 0.3,
        }
    }
}

// ---------------------------------------------------------------------------
// Retry policy
// ---------------------------------------------------------------------------

/// Retry policy that wraps operations with exponential backoff
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    config: RetryConfig,
    operation_name: String,
}

impl RetryPolicy {
    /// Create a new retry policy
    pub fn new(operation_name: impl Into<String>, config: RetryConfig) -> Self {
        Self {
            config,
            operation_name: operation_name.into(),
        }
    }

    /// Execute an operation with retry logic
    pub async fn execute<T, F, Fut>(&self, operation: F) -> Result<T, ApiError>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T, ApiError>>,
    {
        let mut last_error = None;

        for attempt in 0..=self.config.max_retries {
            if attempt > 0 {
                let delay = self.calculate_delay(attempt);
                tracing::info!(
                    "Retrying operation '{}' (attempt {}/{}), waiting {}ms",
                    self.operation_name,
                    attempt,
                    self.config.max_retries,
                    delay.as_millis()
                );
                tokio::time::sleep(delay).await;
                metrics::counter!(
                    "retry_attempts_total",
                    "operation" => self.operation_name.clone()
                )
                .increment(1);
            }

            match operation().await {
                Ok(result) => {
                    if attempt > 0 {
                        tracing::info!(
                            "Operation '{}' succeeded after {} retries",
                            self.operation_name,
                            attempt
                        );
                        metrics::counter!(
                            "retry_successes_total",
                            "operation" => self.operation_name.clone()
                        )
                        .increment(1);
                    }
                    return Ok(result);
                }
                Err(e) => {
                    // Only retry on transient errors
                    if !Self::is_retryable(&e) {
                        tracing::warn!(
                            "Operation '{}' failed with non-retryable error: {}",
                            self.operation_name,
                            e
                        );
                        return Err(e);
                    }

                    tracing::warn!(
                        "Operation '{}' failed (attempt {}/{}): {}",
                        self.operation_name,
                        attempt,
                        self.config.max_retries,
                        e
                    );
                    last_error = Some(e);
                }
            }
        }

        tracing::error!(
            "Operation '{}' exhausted all {} retries",
            self.operation_name,
            self.config.max_retries
        );
        metrics::counter!(
            "retry_exhausted_total",
            "operation" => self.operation_name.clone()
        )
        .increment(1);
        Err(last_error.unwrap_or_else(|| {
            ApiError::Internal(format!(
                "Operation '{}' failed after retries",
                self.operation_name
            ))
        }))
    }

    /// Calculate delay for a given retry attempt using exponential backoff with jitter
    fn calculate_delay(&self, attempt: u32) -> Duration {
        let base = self.config.base_delay_ms;
        let max = self.config.max_delay_ms;

        // Exponential backoff: base * 2^(attempt-1)
        let exponential = base.saturating_mul(2u64.saturating_pow(attempt.saturating_sub(1)));
        let clamped = exponential.min(max);

        // Add jitter: delay * (1 +/- jitter_factor)
        let jitter = if self.config.jitter_factor > 0.0 {
            let range = clamped as f64 * self.config.jitter_factor;
            let offset = rand::thread_rng().gen_range(-range..=range);
            (clamped as f64 + offset).max(0.0) as u64
        } else {
            clamped
        };

        Duration::from_millis(jitter.min(max))
    }

    /// Check if an error is retryable (transient)
    fn is_retryable(error: &ApiError) -> bool {
        match error {
            // Retry on network/database/internal errors
            ApiError::Internal(_) | ApiError::Database(_) | ApiError::ServiceUnavailable(_) => true,
            // Don't retry on client errors
            ApiError::NotFound(_)
            | ApiError::Unauthorized(_)
            | ApiError::Forbidden(_)
            | ApiError::BadRequest(_)
            | ApiError::Validation(_)
            | ApiError::Conflict(_)
            | ApiError::InvalidCredentials
            | ApiError::TokenExpired
            | ApiError::InvalidToken(_)
            | ApiError::MfaRequired(_) => false,
        }
    }
}

// ---------------------------------------------------------------------------
// Retry statistics
// ---------------------------------------------------------------------------

/// Global retry statistics tracker
#[derive(Debug, Default)]
pub struct RetryStats {
    total_attempts: std::sync::atomic::AtomicU64,
    total_successes: std::sync::atomic::AtomicU64,
    total_exhausted: std::sync::atomic::AtomicU64,
}

impl RetryStats {
    /// Create new retry stats tracker
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a retry attempt
    pub fn record_attempt(&self) {
        self.total_attempts
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// Record a successful retry
    pub fn record_success(&self) {
        self.total_successes
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// Record an exhausted retry
    pub fn record_exhausted(&self) {
        self.total_exhausted
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    /// Get current stats
    pub fn snapshot(&self) -> RetryStatsSnapshot {
        RetryStatsSnapshot {
            total_attempts: self
                .total_attempts
                .load(std::sync::atomic::Ordering::Relaxed),
            total_successes: self
                .total_successes
                .load(std::sync::atomic::Ordering::Relaxed),
            total_exhausted: self
                .total_exhausted
                .load(std::sync::atomic::Ordering::Relaxed),
        }
    }
}

/// Snapshot of retry statistics
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RetryStatsSnapshot {
    pub total_attempts: u64,
    pub total_successes: u64,
    pub total_exhausted: u64,
}

/// Type alias for shared retry stats
pub type BoxRetryStats = Arc<RetryStats>;

// ---------------------------------------------------------------------------
// Combined resilient wrapper
// ---------------------------------------------------------------------------

/// Execute an operation with both circuit breaker and retry protection
pub async fn resilient_call<T, F, Fut>(
    circuit_breaker: &crate::common::circuit_breaker::CircuitBreaker,
    retry_policy: &RetryPolicy,
    operation: F,
) -> Result<T, ApiError>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T, ApiError>>,
{
    circuit_breaker
        .call(|| async { retry_policy.execute(&operation).await })
        .await
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    #[test]
    fn test_retry_config_defaults() {
        let config = RetryConfig::default();
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.base_delay_ms, 500);
        assert_eq!(config.max_delay_ms, 30_000);
        assert!(config.jitter_factor >= 0.0 && config.jitter_factor <= 1.0);
    }

    #[test]
    fn test_calculate_delay() {
        let policy = RetryPolicy::new("test", RetryConfig::default());

        // Attempt 1: base_delay
        let d1 = policy.calculate_delay(1);
        assert!(d1.as_millis() >= 350 && d1.as_millis() <= 650); // with jitter

        // Attempt 2: base * 2
        let d2 = policy.calculate_delay(2);
        assert!(d2.as_millis() >= 700 && d2.as_millis() <= 1300);

        // Attempt 3: base * 4
        let d3 = policy.calculate_delay(3);
        assert!(d3.as_millis() >= 1400 && d3.as_millis() <= 2600);
    }

    #[test]
    fn test_delay_clamped_to_max() {
        let config = RetryConfig {
            max_retries: 10,
            base_delay_ms: 1000,
            max_delay_ms: 5000,
            jitter_factor: 0.0,
        };
        let policy = RetryPolicy::new("test", config);

        // Attempt 10 would be 1000 * 2^9 = 512000, but clamped to 5000
        let d = policy.calculate_delay(10);
        assert_eq!(d.as_millis(), 5000);
    }

    #[tokio::test]
    async fn test_execute_success_first_try() {
        let policy = RetryPolicy::new("test", RetryConfig::default());
        let result = policy.execute(|| async { Ok::<_, ApiError>(42) }).await;
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_execute_retries_then_succeeds() {
        let policy = RetryPolicy::new(
            "test",
            RetryConfig {
                max_retries: 3,
                base_delay_ms: 10,
                max_delay_ms: 100,
                jitter_factor: 0.0,
            },
        );

        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();

        let result = policy
            .execute(move || {
                let c = counter_clone.clone();
                async move {
                    let attempt = c.fetch_add(1, Ordering::SeqCst);
                    if attempt < 2 {
                        Err(ApiError::Internal("transient".to_string()))
                    } else {
                        Ok(42)
                    }
                }
            })
            .await;

        assert_eq!(result.unwrap(), 42);
        assert_eq!(counter.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_execute_non_retryable_error() {
        let policy = RetryPolicy::new(
            "test",
            RetryConfig {
                max_retries: 3,
                base_delay_ms: 10,
                max_delay_ms: 100,
                jitter_factor: 0.0,
            },
        );

        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();

        let result = policy
            .execute(move || {
                let c = counter_clone.clone();
                async move {
                    c.fetch_add(1, Ordering::SeqCst);
                    Err::<(), ApiError>(ApiError::NotFound("not found".to_string()))
                }
            })
            .await;

        assert!(matches!(result, Err(ApiError::NotFound(_))));
        assert_eq!(counter.load(Ordering::SeqCst), 1); // No retries
    }

    #[tokio::test]
    async fn test_execute_exhausts_retries() {
        let policy = RetryPolicy::new(
            "test",
            RetryConfig {
                max_retries: 2,
                base_delay_ms: 10,
                max_delay_ms: 100,
                jitter_factor: 0.0,
            },
        );

        let result = policy
            .execute(|| async { Err::<i32, _>(ApiError::Internal("always fails".to_string())) })
            .await;

        assert!(matches!(result, Err(ApiError::Internal(_))));
    }

    #[tokio::test]
    async fn test_retryable_service_unavailable() {
        let policy = RetryPolicy::new(
            "test",
            RetryConfig {
                max_retries: 1,
                base_delay_ms: 10,
                max_delay_ms: 100,
                jitter_factor: 0.0,
            },
        );

        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();

        let result = policy
            .execute(move || {
                let c = counter_clone.clone();
                async move {
                    c.fetch_add(1, Ordering::SeqCst);
                    Err::<(), ApiError>(ApiError::ServiceUnavailable("down".to_string()))
                }
            })
            .await;

        assert!(matches!(result, Err(ApiError::ServiceUnavailable(_))));
        assert_eq!(counter.load(Ordering::SeqCst), 2); // Initial + 1 retry
    }

    #[test]
    fn test_retry_stats() {
        let stats = RetryStats::new();
        stats.record_attempt();
        stats.record_attempt();
        stats.record_success();
        stats.record_exhausted();

        let snapshot = stats.snapshot();
        assert_eq!(snapshot.total_attempts, 2);
        assert_eq!(snapshot.total_successes, 1);
        assert_eq!(snapshot.total_exhausted, 1);
    }
}
