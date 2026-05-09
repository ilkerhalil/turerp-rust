//! Resilience API endpoints
//!
//! Provides monitoring and control for circuit breakers and retry statistics.

use actix_web::{get, post, web, HttpResponse};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::common::circuit_breaker::{CircuitBreakerRegistry, CircuitBreakerStats};
use crate::common::retry::RetryStatsSnapshot;
use crate::error::ApiError;

// ---------------------------------------------------------------------------
// DTOs
// ---------------------------------------------------------------------------

/// Response for listing all circuit breakers
#[derive(Debug, Serialize, ToSchema)]
pub struct CircuitBreakerListResponse {
    pub circuit_breakers: Vec<CircuitBreakerStats>,
}

/// Response for a single circuit breaker reset
#[derive(Debug, Serialize, ToSchema)]
pub struct CircuitBreakerResetResponse {
    pub message: String,
    pub service: String,
}

/// Response for retry statistics
#[derive(Debug, Serialize, ToSchema)]
pub struct RetryStatsResponse {
    pub retry_stats: RetryStatsSnapshot,
}

/// Path parameter for service name
#[derive(Debug, Deserialize, ToSchema)]
pub struct ServicePath {
    pub service: String,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// List all circuit breaker states
///
/// Returns the current state, failure counts, and statistics for all
/// registered circuit breakers (GIB, email, SMS, bank, webhook, file storage).
#[utoipa::path(
    get,
    path = "/api/v1/resilience/circuit-breakers",
    tag = "Resilience",
    responses(
        (status = 200, description = "Circuit breaker states", body = CircuitBreakerListResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
    ),
    security(("bearer_auth" = [])),
)]
#[get("/resilience/circuit-breakers")]
pub async fn list_circuit_breakers(
    registry: web::Data<CircuitBreakerRegistry>,
) -> Result<HttpResponse, ApiError> {
    let circuit_breakers = registry.list_all();
    Ok(HttpResponse::Ok().json(CircuitBreakerListResponse { circuit_breakers }))
}

/// Reset a specific circuit breaker
///
/// Manually resets the circuit breaker for the given service to Closed state.
/// Useful after resolving an external service outage.
#[utoipa::path(
    post,
    path = "/api/v1/resilience/circuit-breakers/{service}/reset",
    tag = "Resilience",
    params(
        ("service" = String, Path, description = "Service name (gib, email, sms, bank, webhook, file_storage)")
    ),
    responses(
        (status = 200, description = "Circuit breaker reset", body = CircuitBreakerResetResponse),
        (status = 400, description = "Unknown service", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
    ),
    security(("bearer_auth" = [])),
)]
#[post("/resilience/circuit-breakers/{service}/reset")]
pub async fn reset_circuit_breaker(
    registry: web::Data<CircuitBreakerRegistry>,
    path: web::Path<ServicePath>,
) -> Result<HttpResponse, ApiError> {
    if registry.reset(&path.service) {
        Ok(HttpResponse::Ok().json(CircuitBreakerResetResponse {
            message: format!("Circuit breaker for '{}' has been reset", path.service),
            service: path.service.clone(),
        }))
    } else {
        Err(ApiError::BadRequest(format!(
            "Unknown circuit breaker service: {}. Valid services: gib, email, sms, bank, webhook, file_storage",
            path.service
        )))
    }
}

/// Get retry statistics
///
/// Returns global retry attempt, success, and exhaustion counters.
#[utoipa::path(
    get,
    path = "/api/v1/resilience/retry-stats",
    tag = "Resilience",
    responses(
        (status = 200, description = "Retry statistics", body = RetryStatsResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse),
        (status = 403, description = "Forbidden", body = ErrorResponse),
    ),
    security(("bearer_auth" = [])),
)]
#[get("/resilience/retry-stats")]
pub async fn get_retry_stats(
    retry_stats: web::Data<crate::common::retry::BoxRetryStats>,
) -> Result<HttpResponse, ApiError> {
    let snapshot = retry_stats.snapshot();
    Ok(HttpResponse::Ok().json(RetryStatsResponse {
        retry_stats: snapshot,
    }))
}

// ---------------------------------------------------------------------------
// Route configuration
// ---------------------------------------------------------------------------

/// Configure resilience routes
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(list_circuit_breakers)
        .service(reset_circuit_breaker)
        .service(get_retry_stats);
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_path() {
        let sp = ServicePath {
            service: "gib".to_string(),
        };
        assert_eq!(sp.service, "gib");
    }
}
