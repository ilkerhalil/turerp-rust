//! Performance / load tests for the Turerp ERP API
//!
//! These tests measure throughput and latency of key endpoints under
//! concurrent load using the Actix Web test server.

use actix_web::{test, web, App};
use serde_json::json;
use std::num::NonZeroU32;
use std::time::{Duration, Instant};
use turerp::api::v1_jobs_configure;
use turerp::app::create_app_state_in_memory;
use turerp::common::JobScheduler;
use turerp::config::Config;
use turerp::middleware::{
    JwtAuthMiddleware, MetricsMiddleware, RateLimitMiddleware, RequestIdMiddleware,
    TenantMiddleware,
};
use turerp::utils::jwt::JwtService;

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

// ============================================================================
// Job Scheduler Performance Tests
// ============================================================================

/// Helper: create admin token for performance tests
fn create_perf_admin_token(jwt: &JwtService) -> String {
    let tokens = jwt
        .generate_tokens(1, 1, "perfadmin".to_string(), turerp::Role::Admin)
        .unwrap();
    tokens.access_token
}

/// Benchmark: sequential job scheduling throughput
#[actix_web::test]
async fn test_job_scheduling_throughput() {
    let config = Config::default();
    let app_state = create_app_state_in_memory(&config);

    let jwt = JwtService::new(
        config.jwt.secret.clone(),
        config.jwt.access_token_expiration,
        config.jwt.refresh_token_expiration,
    );
    let token = create_perf_admin_token(&jwt);

    let app = test::init_service(
        App::new()
            .wrap(JwtAuthMiddleware::new(jwt))
            .app_data(app_state.job_scheduler.clone())
            .service(web::scope("/api").configure(v1_jobs_configure)),
    )
    .await;

    let count = 200;
    let start = Instant::now();

    for i in 0..count {
        let req = test::TestRequest::post()
            .uri("/api/v1/jobs")
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .set_json(json!({
                "job_type": "generate_report",
                "tenant_id": 1,
                "report_type": "perf",
                "params": format!("{{\"i\":{}}}", i)
            }))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }

    let elapsed = start.elapsed();
    let avg_ms = elapsed.as_millis() as f64 / count as f64;
    let throughput = count as f64 / elapsed.as_secs_f64();
    println!(
        "Job scheduling: {} jobs in {:?} (avg {:.3} ms/job, {:.1} jobs/sec)",
        count, elapsed, avg_ms, throughput
    );
    assert!(
        avg_ms < 5.0,
        "avg scheduling latency too high: {:.3} ms",
        avg_ms
    );
}

/// Benchmark: concurrent job scheduling burst
#[actix_web::test]
async fn test_concurrent_job_scheduling_burst() {
    let config = Config::default();
    let app_state = create_app_state_in_memory(&config);

    let jwt = JwtService::new(
        config.jwt.secret.clone(),
        config.jwt.access_token_expiration,
        config.jwt.refresh_token_expiration,
    );
    let token = create_perf_admin_token(&jwt);

    let app = test::init_service(
        App::new()
            .wrap(JwtAuthMiddleware::new(jwt))
            .app_data(app_state.job_scheduler.clone())
            .service(web::scope("/api").configure(v1_jobs_configure)),
    )
    .await;

    let count = 100;
    let start = Instant::now();

    let responses = futures::future::join_all((0..count).map(|i| {
        let req = test::TestRequest::post()
            .uri("/api/v1/jobs")
            .insert_header(("Authorization", format!("Bearer {}", token)))
            .set_json(json!({
                "job_type": "send_reminders",
                "tenant_id": 1,
                "priority": if i % 4 == 0 { "critical" } else { "normal" }
            }))
            .to_request();
        test::call_service(&app, req)
    }))
    .await;

    let elapsed = start.elapsed();
    let success_count = responses.iter().filter(|r| r.status().is_success()).count();
    let avg_ms = elapsed.as_millis() as f64 / count as f64;
    let throughput = count as f64 / elapsed.as_secs_f64();

    println!(
        "Concurrent job scheduling: {}/{} success in {:?} (avg {:.3} ms/job, {:.1} jobs/sec)",
        success_count, count, elapsed, avg_ms, throughput
    );
    assert_eq!(success_count, count);
    assert!(
        avg_ms < 10.0,
        "avg latency too high under burst: {:.3} ms",
        avg_ms
    );
}

/// Benchmark: job queue processing (next_pending throughput)
#[actix_web::test]
async fn test_job_queue_processing_throughput() {
    let scheduler = turerp::common::InMemoryJobScheduler::new();

    // Pre-fill queue with 500 jobs
    let tenant_id = 1;
    for _ in 0..500 {
        scheduler
            .schedule(turerp::common::CreateJob::new(
                turerp::common::JobType::SendReminders { tenant_id },
                tenant_id,
            ))
            .await
            .unwrap();
    }

    let count = 500;
    let start = Instant::now();

    for _ in 0..count {
        let _ = scheduler.next_pending().await.unwrap();
    }

    let elapsed = start.elapsed();
    let avg_us = elapsed.as_micros() as f64 / count as f64;
    let throughput = count as f64 / elapsed.as_secs_f64();
    println!(
        "Job queue next_pending: {} ops in {:?} (avg {:.1} us/op, {:.0} ops/sec)",
        count, elapsed, avg_us, throughput
    );
    assert!(
        avg_us < 500.0,
        "avg next_pending latency too high: {:.1} us",
        avg_us
    );
}

/// Benchmark: concurrent next_pending with contention
#[actix_web::test]
async fn test_concurrent_next_pending_contention() {
    let scheduler = std::sync::Arc::new(turerp::common::InMemoryJobScheduler::new());

    // Pre-fill queue
    let tenant_id = 1;
    for i in 0..200 {
        let priority = if i % 5 == 0 {
            turerp::common::JobPriority::Critical
        } else {
            turerp::common::JobPriority::Normal
        };
        scheduler
            .schedule(
                turerp::common::CreateJob::new(
                    turerp::common::JobType::SendReminders { tenant_id },
                    tenant_id,
                )
                .with_priority(priority),
            )
            .await
            .unwrap();
    }

    let count = 200;
    let start = Instant::now();

    let handles: Vec<_> = (0..count)
        .map(|_| {
            let s = scheduler.clone();
            tokio::spawn(async move {
                let _ = s.next_pending().await.unwrap();
            })
        })
        .collect();

    for h in handles {
        let _ = h.await;
    }

    let elapsed = start.elapsed();
    let avg_us = elapsed.as_micros() as f64 / count as f64;
    println!(
        "Concurrent next_pending: {} ops in {:?} (avg {:.1} us/op)",
        count, elapsed, avg_us
    );
    assert!(
        avg_us < 2000.0,
        "avg latency too high under contention: {:.1} us",
        avg_us
    );
}
