//! Regression test for commit `fix(audit): panic recovery + JoinHandle`.
//!
//! `spawn_audit_writer` previously had no panic-recovery wrapper: a single
//! panic in `service.create_batch` killed the writer for the process
//! lifetime, silently dropping every subsequent audit event. The fix wraps
//! the writer tick in `AssertUnwindSafe(...).catch_unwind().await` with
//! exponential backoff, so a transient DB error inside `create_batch`
//! logs the panic, sleeps briefly, and resumes draining from the same
//! channel.
//!
//! This test wires a `PanickingAuditLogRepository` that panics on the
//! first `create_batch` call and then delegates to an in-memory repo on
//! subsequent calls, then asserts that:
//!  - The writer does not exit (JoinHandle never completes) after the
//!    first panic, despite the receiver still being live.
//!  - After the backoff elapses, the next batch is successfully flushed.
//!  - Closing the channel causes a clean drain (JoinHandle completes).

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use turerp::common::pagination::PaginatedResult;
use turerp::domain::audit::model::{AuditLog, AuditLogQueryParams, CreateAuditLog};
use turerp::domain::audit::repository::{
    AuditLogRepository, BoxAuditLogRepository, InMemoryAuditLogRepository,
};
use turerp::domain::audit::service::AuditService;
use turerp::error::ApiError;
use turerp::middleware::audit::{spawn_audit_writer, AuditEvent};

/// Test repo: panics on the first `create_batch` to trigger the
/// supervisor's catch_unwind path, then delegates to an in-memory repo
/// for any subsequent calls. Counts how many successful batches land.
struct PanickingAuditLogRepository {
    delegate: BoxAuditLogRepository,
    panic_calls: Arc<AtomicUsize>,
    success_calls: Arc<AtomicUsize>,
    first_call_started: Arc<AtomicUsize>,
}

#[async_trait]
impl AuditLogRepository for PanickingAuditLogRepository {
    async fn create(&self, log: CreateAuditLog) -> Result<AuditLog, ApiError> {
        self.delegate.create(log).await
    }

    async fn create_batch(&self, logs: Vec<CreateAuditLog>) -> Result<(), ApiError> {
        // First call: mark that the panic path was reached, then panic.
        // Subsequent calls: delegate to the in-memory repo.
        let n = self.first_call_started.fetch_add(1, Ordering::SeqCst);
        if n == 0 {
            self.panic_calls.fetch_add(1, Ordering::SeqCst);
            panic!("simulated transient DB error in create_batch");
        }
        self.success_calls.fetch_add(1, Ordering::SeqCst);
        self.delegate.create_batch(logs).await
    }

    async fn find_by_tenant_paginated(
        &self,
        tenant_id: i64,
        query: AuditLogQueryParams,
    ) -> Result<PaginatedResult<AuditLog>, ApiError> {
        self.delegate
            .find_by_tenant_paginated(tenant_id, query)
            .await
    }
}

fn make_event(tenant_id: i64, n: i64) -> AuditEvent {
    AuditEvent {
        tenant_id,
        user_id: n,
        username: format!("u{}", n),
        action: "POST".to_string(),
        path: "/api/v1/test".to_string(),
        status_code: 200,
        request_id: format!("req-{}", n),
        ip_address: None,
        user_agent: None,
    }
}

#[tokio::test]
async fn audit_writer_recovers_from_panic_and_flushes_next_batch() {
    let panic_calls = Arc::new(AtomicUsize::new(0));
    let success_calls = Arc::new(AtomicUsize::new(0));
    let first_call_started = Arc::new(AtomicUsize::new(0));

    let delegate: BoxAuditLogRepository = Arc::new(InMemoryAuditLogRepository::new());
    let panicking: Arc<PanickingAuditLogRepository> = Arc::new(PanickingAuditLogRepository {
        delegate: delegate.clone(),
        panic_calls: panic_calls.clone(),
        success_calls: success_calls.clone(),
        first_call_started: first_call_started.clone(),
    });
    let repo: BoxAuditLogRepository = panicking.clone();
    let service = AuditService::new(repo);

    let (tx, rx) = tokio::sync::mpsc::channel::<AuditEvent>(1024);
    let handle = spawn_audit_writer(rx, service);

    // Send 100 events — the writer's flush trigger is buffer.len() >= 100.
    // This forces a call to create_batch, which panics on the first call.
    for i in 0..100 {
        if tx.send(make_event(1, i)).await.is_err() {
            break;
        }
    }

    // Wait long enough for: panic, then at least 1s backoff, then a
    // second batch attempt. 1500ms is well above 1s (the minimum
    // backoff is 1s) and well below the 5s flush interval so a
    // successful second batch must come from the 100-event buffer, not
    // from the periodic timer.
    tokio::time::sleep(Duration::from_millis(1500)).await;

    // Send another 100 events to force a second batch attempt that
    // should succeed.
    for i in 100..200 {
        if tx.send(make_event(1, i)).await.is_err() {
            break;
        }
    }

    // Give the writer time to flush the second batch.
    tokio::time::sleep(Duration::from_millis(500)).await;

    // The panic path was reached exactly once.
    assert_eq!(
        panic_calls.load(Ordering::SeqCst),
        1,
        "first create_batch must have panicked"
    );
    // The first 100 events were dropped (panic), but the second 100
    // must have been flushed. So at least one success was recorded.
    assert!(
        success_calls.load(Ordering::SeqCst) >= 1,
        "second batch must have been flushed after panic recovery"
    );

    // Close the channel → writer drains and JoinHandle completes.
    drop(tx);
    let join_result = tokio::time::timeout(Duration::from_secs(2), handle).await;
    assert!(
        join_result.is_ok(),
        "writer JoinHandle must complete after channel close"
    );
}
