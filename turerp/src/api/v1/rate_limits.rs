//! Rate Limit Dashboard API
//!
//! Admin-only endpoint exposing current rate-limit statistics
//! collected by the `RateLimitMiddleware`.

use actix_web::{get, web, HttpResponse};
use serde::Serialize;

use crate::error::ApiError;
use crate::middleware::auth::AdminUser;
use crate::middleware::rate_limit::RateLimitStatsStore;

/// Summary of rate-limit activity for one client IP
#[derive(Serialize, utoipa::ToSchema)]
pub struct RateLimitEntryResponse {
    pub client_key: String,
    pub total_requests: u64,
    pub blocked_requests: u64,
    pub allowed_requests: u64,
    #[schema(value_type = Option<String>)]
    pub last_request_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Overall dashboard response
#[derive(Serialize, utoipa::ToSchema)]
pub struct RateLimitDashboardResponse {
    pub total_clients: usize,
    pub total_requests: u64,
    pub total_blocked: u64,
    pub total_allowed: u64,
    pub entries: Vec<RateLimitEntryResponse>,
}

/// Get rate limit statistics (admin only)
#[utoipa::path(
    get,
    path = "/api/v1/admin/rate-limits",
    responses(
        (status = 200, description = "Rate limit statistics", body = RateLimitDashboardResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - admin only"),
    ),
    tag = "Admin"
)]
#[get("/admin/rate-limits")]
pub async fn get_rate_limit_stats(
    _admin: AdminUser,
    stats_store: web::Data<RateLimitStatsStore>,
) -> Result<HttpResponse, ApiError> {
    let map = stats_store.read();

    let mut entries = Vec::with_capacity(map.len());
    let mut total_requests = 0u64;
    let mut total_blocked = 0u64;

    for (client_key, stats) in map.iter() {
        total_requests += stats.total_requests;
        total_blocked += stats.blocked_requests;

        let last_request_at = stats.last_request_at.map(|t| {
            let duration = t.duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
            chrono::DateTime::from_timestamp(duration.as_secs() as i64, 0)
                .unwrap_or(chrono::DateTime::UNIX_EPOCH)
        });

        entries.push(RateLimitEntryResponse {
            client_key: client_key.clone(),
            total_requests: stats.total_requests,
            blocked_requests: stats.blocked_requests,
            allowed_requests: stats.total_requests.saturating_sub(stats.blocked_requests),
            last_request_at,
        });
    }

    // Sort by total requests descending
    entries.sort_by_key(|b| std::cmp::Reverse(b.total_requests));

    Ok(HttpResponse::Ok().json(RateLimitDashboardResponse {
        total_clients: entries.len(),
        total_requests,
        total_blocked,
        total_allowed: total_requests.saturating_sub(total_blocked),
        entries,
    }))
}

/// Configure rate-limit dashboard routes
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(get_rate_limit_stats);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_rate_limit_entry_response_sorting() {
        let mut entries = vec![
            RateLimitEntryResponse {
                client_key: "a".to_string(),
                total_requests: 10,
                blocked_requests: 2,
                allowed_requests: 8,
                last_request_at: None,
            },
            RateLimitEntryResponse {
                client_key: "b".to_string(),
                total_requests: 50,
                blocked_requests: 5,
                allowed_requests: 45,
                last_request_at: None,
            },
        ];
        entries.sort_by_key(|b| std::cmp::Reverse(b.total_requests));
        assert_eq!(entries[0].client_key, "b");
        assert_eq!(entries[1].client_key, "a");
    }

    #[test]
    fn test_rate_limit_dashboard_response_totals() {
        let resp = RateLimitDashboardResponse {
            total_clients: 2,
            total_requests: 60,
            total_blocked: 7,
            total_allowed: 53,
            entries: vec![],
        };
        assert_eq!(resp.total_clients, 2);
        assert_eq!(resp.total_allowed, 60 - 7);
    }
}
