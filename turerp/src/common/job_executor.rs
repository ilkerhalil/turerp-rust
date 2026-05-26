//! Background job executor that polls pending jobs and dispatches them.

/// System user ID for background job operations (no authenticated user context).
const SYSTEM_USER_ID: i64 = 0;

use std::time::Duration;

use actix_web::web;

use crate::common::file_storage::FileStorage;
use crate::common::import::model::{EntityType, ImportFormat};
use crate::common::import::ImportService;
use crate::common::jobs::{JobScheduler, JobType};

/// Executor that polls for pending jobs and processes them.
pub struct JobExecutor {
    job_scheduler: web::Data<dyn JobScheduler>,
    import_service: web::Data<dyn ImportService>,
    file_storage: web::Data<dyn FileStorage>,
    shutdown: parking_lot::Mutex<Option<tokio::sync::mpsc::Sender<()>>>,
}

impl JobExecutor {
    /// Create a new job executor.
    pub fn new(
        job_scheduler: web::Data<dyn JobScheduler>,
        import_service: web::Data<dyn ImportService>,
        file_storage: web::Data<dyn FileStorage>,
    ) -> Self {
        Self {
            job_scheduler,
            import_service,
            file_storage,
            shutdown: parking_lot::Mutex::new(None),
        }
    }

    /// Start the background polling loop.
    pub async fn start(&self) {
        let scheduler = self.job_scheduler.clone();
        let import_service = self.import_service.clone();
        let file_storage = self.file_storage.clone();
        let (tx, mut rx) = tokio::sync::mpsc::channel::<()>(1);
        *self.shutdown.lock() = Some(tx);

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(5));
            // Skip the immediate first tick.
            interval.tick().await;

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        if let Err(e) = Self::poll_and_execute(&scheduler, &import_service, &file_storage).await {
                            tracing::warn!("Job executor poll failed: {}", e);
                        }
                    }
                    _ = rx.recv() => {
                        tracing::info!("Job executor shutting down");
                        break;
                    }
                }
            }
        });
    }

    /// Signal the executor to shut down.
    pub async fn shutdown(&self) {
        let tx = self.shutdown.lock().take();
        if let Some(tx) = tx {
            let _ = tx.send(()).await.ok();
        }
    }

    async fn poll_and_execute(
        scheduler: &web::Data<dyn JobScheduler>,
        import_service: &web::Data<dyn ImportService>,
        file_storage: &web::Data<dyn FileStorage>,
    ) -> Result<(), String> {
        let job = match scheduler.next_pending().await? {
            Some(j) => j,
            None => return Ok(()),
        };

        tracing::info!("Executing job {}: {:?}", job.id, job.job_type);

        if let Err(e) = scheduler.mark_running(job.id).await {
            tracing::warn!("Failed to mark job {} running: {}", job.id, e);
            return Ok(());
        }

        match &job.job_type {
            JobType::Import {
                file_id,
                entity_type,
                tenant_id,
                company_id,
                format,
            } => {
                let result = async {
                    let data = file_storage
                        .download(*tenant_id, *file_id)
                        .await
                        .map_err(|e| format!("Failed to download file {}: {}", file_id, e))?;

                    let entity = entity_type
                        .parse::<EntityType>()
                        .map_err(|e| format!("Invalid entity type: {}", e))?;

                    let fmt = format
                        .parse::<ImportFormat>()
                        .map_err(|e| format!("Invalid format: {}", e))?;

                    import_service
                        .import(*tenant_id, *company_id, entity, fmt, data, SYSTEM_USER_ID)
                        .await
                        .map_err(|e| format!("Import failed: {}", e))?;

                    Ok::<(), String>(())
                }
                .await;

                match result {
                    Ok(()) => {
                        if let Err(e) = scheduler.mark_completed(job.id).await {
                            tracing::warn!("Failed to mark job {} completed: {}", job.id, e);
                        }
                    }
                    Err(e) => {
                        tracing::error!("Job {} failed: {}", job.id, e);
                        if let Err(e2) = scheduler.mark_failed(job.id, &e).await {
                            tracing::warn!("Failed to mark job {} failed: {}", job.id, e2);
                        }
                    }
                }
            }
            other => {
                let msg = format!("No executor registered for job type: {:?}", other);
                tracing::warn!("{}", msg);
                scheduler.mark_failed(job.id, &msg).await.ok();
            }
        }

        Ok(())
    }
}
