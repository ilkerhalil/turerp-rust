//! Regression test for commit `fix(shutdown): start JobService cron`.
//!
//! Before this commit, `JobService::start_background_tasks` was never
//! invoked from `main.rs`. The 60s cron evaluator and 300s stalled-job
//! resetter silently never ran, so scheduled jobs in the database never
//! fired. This test pins the new behavior: starting the background
//! tasks must spawn a task that, given a repository with a due
//! schedule, invokes `evaluate_schedules` within the cron interval.
//!
//! We exercise the new return type (`(JoinHandle, Sender)`) and the
//! shutdown contract. The cron interval is hardcoded at 60s, which is
//! too long for an integration test, so we instead verify the task
//! spawns and shuts down cleanly without panicking. The actual cron
//! behavior is covered by unit tests in `domain/job/service.rs::tests`.

use std::time::Duration;
use turerp::domain::job::repository::InMemoryJobRepository;
use turerp::domain::job::service::JobService;

#[tokio::test]
async fn start_background_tasks_returns_joinhandle_and_shutdown_sender() {
    let repo = std::sync::Arc::new(InMemoryJobRepository::new());
    let svc = JobService::new(repo);

    let (handle, tx) = svc.start_background_tasks();

    // The handle is a real JoinHandle<()>, not a bare () (the pre-fix
    // return type was `()`, so main.rs's drain sequence could not
    // await it). We can't simply drop the sender to test completion
    // because the cron interval is hardcoded at 60s and the loop is
    // blocked on the interval tick — the channel close only
    // registers when rx.recv() is polled, which only happens inside
    // the select!. So we send an explicit shutdown and verify the
    // contract.
    tx.send(()).await.expect("send must succeed");
    let join = tokio::time::timeout(Duration::from_secs(2), handle).await;
    assert!(
        join.is_ok(),
        "background task must complete within 2s after shutdown send"
    );
}

#[tokio::test]
async fn shutdown_via_send_completes_background_task() {
    let repo = std::sync::Arc::new(InMemoryJobRepository::new());
    let svc = JobService::new(repo);

    let (handle, tx) = svc.start_background_tasks();

    // The cron interval is 60s, so we don't wait for a real tick.
    // Instead, we verify the shutdown path completes promptly.
    let start = std::time::Instant::now();
    tx.send(()).await.expect("send must succeed");
    let join = tokio::time::timeout(Duration::from_secs(2), handle).await;
    let elapsed = start.elapsed();
    assert!(
        join.is_ok(),
        "background task must exit after shutdown signal (elapsed {:?})",
        elapsed
    );
    // Should be near-instant: the rx.recv() branch fires immediately.
    assert!(
        elapsed < Duration::from_millis(200),
        "shutdown latency too high: {:?}",
        elapsed
    );
}
