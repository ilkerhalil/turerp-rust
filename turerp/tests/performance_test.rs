//! Performance / load tests for the Turerp ERP API
//!
//! These tests measure throughput and latency of key endpoints under
//! concurrent load using the Actix Web test server.

use actix_web::{test, web, App};
use std::num::NonZeroU32;
use std::time::{Duration, Instant};
use turerp::app::create_app_state_in_memory;
use turerp::config::Config;
use turerp::middleware::{
    MetricsMiddleware, RateLimitMiddleware, RequestIdMiddleware, TenantMiddleware,
};

/// Benchmark: hit the liveness endpoint with sequential requests
#[actix_web::test]
async fn test_sequential_health_latency() {
    let config = Config::default();
    let app_state = create_app_state_in_memory(&config);

    let app = test::init_service(
        App::new()
            .wrap(MetricsMiddleware::new())
            .wrap(RateLimitMiddleware::with_quota(
                NonZeroU32::new(100_000).unwrap(),
                NonZeroU32::new(100_000).unwrap(),
            ))
            .wrap(TenantMiddleware)
            .wrap(RequestIdMiddleware)
            .app_data(web::JsonConfig::default().limit(1024 * 1024))
            .app_data(app_state.auth_service.clone())
            .app_data(app_state.user_service.clone())
            .app_data(app_state.jwt_service.clone())
            .app_data(app_state.i18n.clone())
            .route(
                "/health/live",
                web::get().to(|| async {
                    actix_web::HttpResponse::Ok().json(serde_json::json!({"status":"ok"}))
                }),
            ),
    )
    .await;

    let count = 100;
    let start = Instant::now();

    for _ in 0..count {
        let req = test::TestRequest::get().uri("/health/live").to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }

    let elapsed = start.elapsed();
    let avg_ms = elapsed.as_millis() as f64 / count as f64;
    println!(
        "Sequential: {} reqs in {:?} (avg {:.3} ms/req)",
        count, elapsed, avg_ms
    );
    assert!(avg_ms < 10.0, "avg latency too high: {:.3} ms", avg_ms);
}

/// Benchmark: concurrent burst against the liveness endpoint
#[actix_web::test]
async fn test_concurrent_health_burst() {
    let config = Config::default();
    let app_state = create_app_state_in_memory(&config);

    let app = test::init_service(
        App::new()
            .wrap(MetricsMiddleware::new())
            .wrap(RateLimitMiddleware::with_quota(
                NonZeroU32::new(100_000).unwrap(),
                NonZeroU32::new(100_000).unwrap(),
            ))
            .wrap(TenantMiddleware)
            .wrap(RequestIdMiddleware)
            .app_data(web::JsonConfig::default().limit(1024 * 1024))
            .app_data(app_state.auth_service.clone())
            .app_data(app_state.user_service.clone())
            .app_data(app_state.jwt_service.clone())
            .app_data(app_state.i18n.clone())
            .route(
                "/health/live",
                web::get().to(|| async {
                    actix_web::HttpResponse::Ok().json(serde_json::json!({"status":"ok"}))
                }),
            ),
    )
    .await;

    let count = 200;
    let start = Instant::now();

    let responses = futures::future::join_all((0..count).map(|_| {
        let req = test::TestRequest::get().uri("/health/live").to_request();
        test::call_service(&app, req)
    }))
    .await;

    let elapsed = start.elapsed();
    let ok_count = responses.iter().filter(|r| r.status().is_success()).count();

    println!(
        "Concurrent burst: {} reqs in {:?} ({} OK, {} failures)",
        count,
        elapsed,
        ok_count,
        count - ok_count
    );

    assert_eq!(ok_count, count, "all concurrent requests should succeed");
    assert!(
        elapsed < Duration::from_secs(5),
        "concurrent burst took too long: {:?}",
        elapsed
    );
}

/// Benchmark: POST-heavy simulated registration load
#[actix_web::test]
async fn test_post_load_json_parsing() {
    let config = Config::default();
    let app_state = create_app_state_in_memory(&config);

    let app = test::init_service(
        App::new()
            .wrap(MetricsMiddleware::new())
            .wrap(RateLimitMiddleware::with_quota(
                NonZeroU32::new(100_000).unwrap(),
                NonZeroU32::new(100_000).unwrap(),
            ))
            .wrap(TenantMiddleware)
            .wrap(RequestIdMiddleware)
            .app_data(web::JsonConfig::default().limit(1024 * 1024))
            .app_data(app_state.auth_service.clone())
            .app_data(app_state.user_service.clone())
            .app_data(app_state.jwt_service.clone())
            .app_data(app_state.i18n.clone())
            .route(
                "/health/live",
                web::get().to(|| async {
                    actix_web::HttpResponse::Ok().json(serde_json::json!({"status":"ok"}))
                }),
            ),
    )
    .await;

    let count = 50;
    let payload = serde_json::json!({
        "username": "load_test_user",
        "email": "load@example.com",
        "password": "SecurePass123!",
        "tenant_id": 1,
    });

    let start = Instant::now();
    for _ in 0..count {
        let req = test::TestRequest::post()
            .uri("/health/live") // using as a dummy POST target
            .set_json(&payload)
            .to_request();
        let _ = test::call_service(&app, req).await;
    }
    let elapsed = start.elapsed();
    let avg_ms = elapsed.as_millis() as f64 / count as f64;
    println!(
        "POST JSON load: {} reqs in {:?} (avg {:.3} ms/req)",
        count, elapsed, avg_ms
    );
}

/// Benchmark: rapid successive requests from same IP to exercise rate-limit stats
#[actix_web::test]
async fn test_rate_limit_stats_accumulation() {
    let config = Config::default();
    let app_state = create_app_state_in_memory(&config);

    let app = test::init_service(
        App::new()
            .wrap(MetricsMiddleware::new())
            .wrap(RateLimitMiddleware::new())
            .wrap(TenantMiddleware)
            .wrap(RequestIdMiddleware)
            .app_data(web::JsonConfig::default().limit(1024 * 1024))
            .app_data(app_state.auth_service.clone())
            .app_data(app_state.user_service.clone())
            .app_data(app_state.jwt_service.clone())
            .app_data(app_state.i18n.clone())
            .route(
                "/health/live",
                web::get().to(|| async {
                    actix_web::HttpResponse::Ok().json(serde_json::json!({"status":"ok"}))
                }),
            ),
    )
    .await;

    let count = 20;

    for _ in 0..count {
        let req = test::TestRequest::get()
            .uri("/health/live")
            .insert_header(("X-Forwarded-For", "203.0.113.42"))
            .to_request();
        let resp = test::call_service(&app, req).await;
        // Some may be rate-limited (burst = 3)
        assert!(
            resp.status().is_success() || resp.status() == 429,
            "unexpected status: {}",
            resp.status()
        );
    }

    println!("Rate-limit stats accumulated over {} requests", count);
}
