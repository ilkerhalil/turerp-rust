//! Job executor shutdown contract tests.
//!
//! These tests exercise the shutdown path of `JobExecutor::start`:
//! the returned `JoinHandle` MUST terminate within a short timeout
//! once a `()` is sent on the returned `mpsc::Sender`. This guards
//! against regressions where the executor task is orphaned on
//! process exit (e.g. when the channel is dropped instead of
//! driven by the main shutdown sequence).

use std::sync::Arc;
use std::time::Duration;
use turerp::common::file_storage::FileStorage;
use turerp::common::file_storage::LocalFileStorage;
use turerp::common::import::service::CsvImportService;
use turerp::common::import::ImportService;
use turerp::common::job_executor::JobExecutor;
use turerp::common::jobs::{InMemoryJobScheduler, JobScheduler};

async fn build_executor() -> JobExecutor {
    let scheduler: Arc<dyn JobScheduler> = Arc::new(InMemoryJobScheduler::new());
    let product_repo: turerp::domain::product::BoxProductRepository =
        Arc::new(turerp::domain::product::InMemoryProductRepository::new());
    let cari_repo: turerp::domain::cari::BoxCariRepository =
        Arc::new(turerp::domain::cari::InMemoryCariRepository::new());
    let chart_repo: turerp::domain::chart_of_accounts::BoxChartAccountRepository =
        Arc::new(turerp::domain::chart_of_accounts::InMemoryChartAccountRepository::new());
    let stock_movement_repo: turerp::domain::stock::BoxStockMovementRepository =
        Arc::new(turerp::domain::stock::InMemoryStockMovementRepository::new());

    let import: Arc<dyn ImportService> = Arc::new(CsvImportService::new(
        product_repo,
        cari_repo,
        chart_repo,
        stock_movement_repo,
        scheduler.clone(),
    ));

    // File storage: a tempdir-backed local store. The base path
    // is created on first call; we never actually write to it
    // because the in-memory scheduler has no jobs to process.
    let storage: Arc<dyn FileStorage> =
        Arc::new(LocalFileStorage::new(std::env::temp_dir().join("turerp-shutdown-test")).await);

    JobExecutor::new(
        actix_web::web::Data::from(scheduler),
        actix_web::web::Data::from(import),
        actix_web::web::Data::from(storage),
    )
}

#[tokio::test]
async fn job_executor_stops_on_shutdown_signal() {
    let executor = build_executor().await;
    let (handle, tx) = executor.start();

    // Give the loop a tick to enter its `select!` arm.
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Signal shutdown. The mpsc::Sender is bounded(1) so the send
    // completes as soon as the channel has space — it does not
    // depend on the receiver reading.
    tx.send(())
        .await
        .expect("send should not fail with capacity 1");

    // The handle must complete within 2s. If it does not, the loop
    // is broken and SIGTERM will eventually force-exit the process
    // mid-flight, which is the bug we are guarding against.
    let result = tokio::time::timeout(Duration::from_secs(2), handle).await;
    assert!(
        result.is_ok(),
        "JobExecutor::start loop did not exit within 2s of shutdown signal"
    );
}

#[tokio::test]
async fn job_executor_shutdown_is_idempotent() {
    let executor = build_executor().await;
    let (handle, tx) = executor.start();

    tokio::time::sleep(Duration::from_millis(50)).await;

    // Multiple sends are safe: the channel has capacity 1, the
    // second send only returns Err(()) if the receiver was dropped
    // — neither outcome is fatal here.
    let _ = tx.send(()).await;
    let _ = tx.send(()).await;

    let result = tokio::time::timeout(Duration::from_secs(2), handle).await;
    assert!(result.is_ok());
}
