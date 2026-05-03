//! Audit logging middleware
//!
//! Logs request method, path, user info, and response status for
//! authenticated requests. Sends audit events through a channel for
//! non-blocking batch persistence to the audit log repository.

use actix_web::body::MessageBody;
use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error,
};
use futures::future::{ok, LocalBoxFuture, Ready};
use std::sync::Arc;
use tokio::sync::mpsc;

use crate::domain::audit::model::CreateAuditLog;
use crate::middleware::auth::get_auth_claims;

/// Audit event sent through the channel
#[derive(Debug, Clone)]
pub struct AuditEvent {
    pub tenant_id: i64,
    pub user_id: i64,
    pub username: String,
    pub action: String,
    pub path: String,
    pub status_code: i16,
    pub request_id: String,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
}

/// Maximum number of audit events buffered in the channel before backpressure.
pub const AUDIT_CHANNEL_CAPACITY: usize = 10_000;

/// Audit logging middleware factory
pub struct AuditLoggingMiddleware {
    sender: Option<Arc<mpsc::Sender<AuditEvent>>>,
}

impl Default for AuditLoggingMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

impl AuditLoggingMiddleware {
    /// Create without persistence (just logs to tracing)
    pub fn new() -> Self {
        Self { sender: None }
    }

    /// Create with channel sender for persisting audit logs
    pub fn with_sender(sender: Arc<mpsc::Sender<AuditEvent>>) -> Self {
        Self {
            sender: Some(sender),
        }
    }
}

impl<S, B> Transform<S, ServiceRequest> for AuditLoggingMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Transform = AuditLoggingMiddlewareService<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(AuditLoggingMiddlewareService {
            service,
            sender: self.sender.clone(),
        })
    }
}

/// Audit logging middleware service
pub struct AuditLoggingMiddlewareService<S> {
    service: S,
    sender: Option<Arc<mpsc::Sender<AuditEvent>>>,
}

impl<S, B> Service<ServiceRequest> for AuditLoggingMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let method = req.method().to_string();
        let path = req.path().to_string();
        let request_id = req
            .headers()
            .get("X-Request-ID")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();
        let ip_address = req.connection_info().peer_addr().map(|s| s.to_string());
        let user_agent = req
            .headers()
            .get("User-Agent")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
        let sender = self.sender.clone();

        let fut = self.service.call(req);

        Box::pin(async move {
            let response = fut.await?;

            let status = response.status();

            // Read auth claims from the response's request reference.
            // This allows JwtAuthMiddleware to be placed inner relative to Audit,
            // since claims are injected during request processing and persist
            // through the response chain.
            let auth_info = get_auth_claims(response.request()).ok();

            if let Some(claims) = auth_info {
                let event = AuditEvent {
                    tenant_id: claims.tenant_id,
                    user_id: claims.sub.parse().unwrap_or(0),
                    username: claims.sub.clone(),
                    action: method,
                    path,
                    status_code: status.as_u16() as i16,
                    request_id,
                    ip_address,
                    user_agent,
                };

                if status.is_client_error() || status.is_server_error() {
                    tracing::warn!(
                        tenant_id = %event.tenant_id,
                        user_id = %event.user_id,
                        action = %event.action,
                        path = %event.path,
                        status = %event.status_code,
                        "Request completed with error"
                    );
                } else {
                    tracing::info!(
                        tenant_id = %event.tenant_id,
                        user_id = %event.user_id,
                        action = %event.action,
                        path = %event.path,
                        status = %event.status_code,
                        "Request completed"
                    );
                }

                // Send event to channel for persistence (drop if backpressure exceeded)
                if let Some(sender) = sender {
                    if let Err(e) = sender.try_send(event) {
                        tracing::warn!("Audit log channel full, dropping event: {}", e);
                    }
                }
            } else {
                tracing::debug!(
                    method = %method,
                    path = %path,
                    status = %status.as_u16(),
                    "Unauthenticated request completed"
                );
            }

            Ok(response)
        })
    }
}

/// Maximum retry attempts for audit log flush failures.
const MAX_FLUSH_RETRIES: u32 = 3;

/// Spawn a background task that drains audit events and batch-writes them
pub fn spawn_audit_writer(
    receiver: mpsc::Receiver<AuditEvent>,
    service: crate::domain::audit::service::AuditService,
) {
    tokio::spawn(async move {
        audit_writer_task(receiver, service).await;
    });
}

async fn audit_writer_task(
    mut receiver: mpsc::Receiver<AuditEvent>,
    service: crate::domain::audit::service::AuditService,
) {
    let mut buffer: Vec<CreateAuditLog> = Vec::new();
    let mut dead_letter_queue: Vec<CreateAuditLog> = Vec::new();
    let mut flush_interval = tokio::time::interval(tokio::time::Duration::from_secs(5));

    loop {
        tokio::select! {
            Some(event) = receiver.recv() => {
                let log = CreateAuditLog {
                    tenant_id: event.tenant_id,
                    user_id: event.user_id,
                    username: event.username,
                    action: event.action,
                    path: event.path,
                    status_code: event.status_code,
                    request_id: event.request_id,
                    ip_address: event.ip_address,
                    user_agent: event.user_agent,
                    created_at: chrono::Utc::now(),
                };
                buffer.push(log);

                // Flush if we've accumulated enough
                if buffer.len() >= 100 {
                    if let Err(failed) = flush_buffer_with_retry(&service, &mut buffer, MAX_FLUSH_RETRIES).await {
                        dead_letter_queue.extend(failed);
                        if dead_letter_queue.len() > 1_000 {
                            tracing::error!("Audit DLQ exceeded 1000 items, dropping oldest {} logs", dead_letter_queue.len() - 500);
                            dead_letter_queue.drain(0..dead_letter_queue.len() - 500);
                        }
                    }
                }
            }
            _ = flush_interval.tick() => {
                // Periodic flush
                if !buffer.is_empty() {
                    if let Err(failed) = flush_buffer_with_retry(&service, &mut buffer, MAX_FLUSH_RETRIES).await {
                        dead_letter_queue.extend(failed);
                    }
                }
                // Attempt to retry DLQ items on each tick
                if !dead_letter_queue.is_empty() {
                    let mut dlq_buffer = std::mem::take(&mut dead_letter_queue);
                    if let Err(failed) = flush_buffer_with_retry(&service, &mut dlq_buffer, MAX_FLUSH_RETRIES).await {
                        dead_letter_queue.extend(failed);
                        if dead_letter_queue.len() > 1_000 {
                            tracing::error!("Audit DLQ exceeded 1000 items after retry, dropping oldest {} logs", dead_letter_queue.len() - 500);
                            dead_letter_queue.drain(0..dead_letter_queue.len() - 500);
                        }
                    }
                }
            }
        }
    }
}

/// Flush buffer with exponential backoff retry. Returns failed logs on permanent failure.
async fn flush_buffer_with_retry(
    service: &crate::domain::audit::service::AuditService,
    buffer: &mut Vec<CreateAuditLog>,
    max_retries: u32,
) -> Result<(), Vec<CreateAuditLog>> {
    if buffer.is_empty() {
        return Ok(());
    }

    let logs: Vec<CreateAuditLog> = std::mem::take(buffer);

    for attempt in 1..=max_retries {
        match service.create_batch(logs.clone()).await {
            Ok(()) => {
                if attempt > 1 {
                    tracing::info!(
                        "Audit logs flushed successfully after {} retries",
                        attempt - 1
                    );
                }
                return Ok(());
            }
            Err(e) => {
                if attempt < max_retries {
                    let delay = std::time::Duration::from_millis(100 * 2_u64.pow(attempt - 1));
                    tracing::warn!(
                        "Failed to flush audit logs (attempt {}/{}): {}. Retrying in {:?}...",
                        attempt,
                        max_retries,
                        e,
                        delay
                    );
                    tokio::time::sleep(delay).await;
                } else {
                    tracing::error!(
                        "Failed to flush audit logs after {} attempts: {}. Logs moved to DLQ.",
                        max_retries,
                        e
                    );
                }
            }
        }
    }

    Err(logs)
}
