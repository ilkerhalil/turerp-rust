//! Audit logging middleware
//!
//! Logs request method, path, user info, and response status for
//! authenticated requests. Sends audit events through a channel for
//! non-blocking batch persistence to the audit log repository.

use actix_web::body::BoxBody;
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

/// Audit logging middleware factory
pub struct AuditLoggingMiddleware {
    sender: Option<Arc<mpsc::UnboundedSender<AuditEvent>>>,
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
    pub fn with_sender(sender: Arc<mpsc::UnboundedSender<AuditEvent>>) -> Self {
        Self {
            sender: Some(sender),
        }
    }
}

impl<S> Transform<S, ServiceRequest> for AuditLoggingMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<BoxBody>, Error = Error>,
    S::Future: 'static,
{
    type Response = ServiceResponse<BoxBody>;
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
    sender: Option<Arc<mpsc::UnboundedSender<AuditEvent>>>,
}

impl<S> Service<ServiceRequest> for AuditLoggingMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<BoxBody>, Error = Error>,
    S::Future: 'static,
{
    type Response = ServiceResponse<BoxBody>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let method = req.method().to_string();
        let path = req.path().to_string();
        let auth_info = get_auth_claims(req.request()).ok();
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

                // Send event to channel for persistence
                if let Some(sender) = sender {
                    let _ = sender.send(event);
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

/// Spawn a background task that drains audit events and batch-writes them
pub fn spawn_audit_writer(
    receiver: mpsc::UnboundedReceiver<AuditEvent>,
    service: crate::domain::audit::service::AuditService,
) {
    tokio::spawn(async move {
        audit_writer_task(receiver, service).await;
    });
}

async fn audit_writer_task(
    mut receiver: mpsc::UnboundedReceiver<AuditEvent>,
    service: crate::domain::audit::service::AuditService,
) {
    let mut buffer: Vec<CreateAuditLog> = Vec::new();
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
                    flush_buffer(&service, &mut buffer).await;
                }
            }
            _ = flush_interval.tick() => {
                // Periodic flush
                if !buffer.is_empty() {
                    flush_buffer(&service, &mut buffer).await;
                }
            }
        }
    }
}

async fn flush_buffer(
    service: &crate::domain::audit::service::AuditService,
    buffer: &mut Vec<CreateAuditLog>,
) {
    if buffer.is_empty() {
        return;
    }

    let logs: Vec<CreateAuditLog> = std::mem::take(buffer);
    if let Err(e) = service.create_batch(logs).await {
        tracing::error!("Failed to flush audit logs: {}", e);
    }
}
