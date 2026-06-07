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
use futures::FutureExt;
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

/// Decide whether an audit event must be persisted even at the cost
/// of blocking the request. These are events whose loss would be a
/// security incident in its own right (failed logins, MFA challenges,
/// privilege changes, server errors).
pub fn is_sensitive_audit_event(event: &AuditEvent) -> bool {
    if event.status_code >= 500 {
        return true;
    }
    let action = event.action.to_ascii_lowercase();
    let path = event.path.to_ascii_lowercase();
    const SENSITIVE_KEYWORDS: &[&str] = &[
        "auth",
        "login",
        "logout",
        "mfa",
        "totp",
        "role",
        "permission",
        "privilege",
    ];
    SENSITIVE_KEYWORDS
        .iter()
        .any(|kw| action.contains(kw) || path.contains(kw))
}

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

            // Build the audit event even when the request is
            // unauthenticated, as long as the path is sensitive or the
            // response was a 5xx. Brute-force attempts on /auth/login,
            // MFA failures, and any 5xx must reach audit_logs — silent
            // loss of these events is a security incident. Unauth
            // requests on non-sensitive paths are still skipped (they
            // would be 404s on unknown routes or noise on /health).
            let (tenant_id, user_id, username) = match &auth_info {
                Some(claims) => {
                    let uid = claims.sub.parse().unwrap_or_else(|_| {
                        tracing::warn!(
                            "Failed to parse user_id from JWT sub claim: {}",
                            claims.sub
                        );
                        0
                    });
                    (claims.tenant_id, uid, claims.username.clone())
                }
                None => (0_i64, 0_i64, "anonymous".to_string()),
            };

            let event = AuditEvent {
                tenant_id,
                user_id,
                username,
                action: method,
                path,
                status_code: status.as_u16() as i16,
                request_id,
                ip_address,
                user_agent,
            };

            let is_sensitive = is_sensitive_audit_event(&event);
            let is_unauth = auth_info.is_none();
            // Persist if: (a) the event is sensitive (auth/mfa/role/perm
            // path, or any 5xx), or (b) we have auth context. Unauth
            // requests on non-sensitive, non-5xx paths (e.g. 404s on
            // /favicon.ico) are still dropped to keep the audit log
            // signal-to-noise ratio high.
            let should_persist = is_sensitive || !is_unauth;

            if !should_persist {
                tracing::debug!(
                    method = %event.action,
                    path = %event.path,
                    status = %event.status_code,
                    "Unauthenticated non-sensitive request completed (not audited)"
                );
                return Ok(response);
            }

            // The event is held by value in the persist block below;
            // log from the same struct so trace and DB row agree.
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

            // Tiered backpressure: sensitive events (auth, mfa, role,
            // permission, or any 5xx) use a blocking send so they
            // cannot be silently dropped under load. Routine events
            // use try_send to keep the request path latency-bounded.
            if let Some(sender) = sender {
                if is_sensitive {
                    if let Err(e) = sender.send(event).await {
                        tracing::error!("Audit channel closed (sensitive event dropped): {}", e);
                    }
                } else if let Err(e) = sender.try_send(event) {
                    tracing::warn!("Audit log channel full, dropping event: {}", e);
                }
            }

            Ok(response)
        })
    }
}

/// Maximum retry attempts for audit log flush failures.
const MAX_FLUSH_RETRIES: u32 = 3;

/// Maximum panic backoff in seconds (caps the exponential growth).
const MAX_PANIC_BACKOFF_SECS: u64 = 30;

/// Spawn a background task that drains audit events and batch-writes them.
///
/// Returns a `JoinHandle<()>` so the caller can await a clean drain on
/// shutdown. The writer is supervised by a panic-recovery loop: if a panic
/// occurs inside the writer (e.g. a transient DB error in
/// `service.create_batch`), the supervisor catches it, sleeps with
/// exponential backoff, and resumes the writer with the receiver and
/// service still in scope (no ownership transfer into the panicked task).
///
/// Note: the panic backoff sleep here is not preemptible by a shutdown
/// signal; that fix is layered on top in commit 9 (which generalizes the
/// backoff to take a shutdown channel).
pub fn spawn_audit_writer(
    mut receiver: mpsc::Receiver<AuditEvent>,
    service: crate::domain::audit::service::AuditService,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut buffer: Vec<CreateAuditLog> = Vec::new();
        let mut dead_letter_queue: Vec<CreateAuditLog> = Vec::new();
        let mut flush_interval = tokio::time::interval(tokio::time::Duration::from_secs(5));
        let mut panic_backoff_secs: u64 = 0;

        loop {
            // Only the per-iteration body is wrapped in catch_unwind.
            // Receiver + buffer + service stay owned by the outer
            // supervisor, so a panic inside `service.create_batch` does
            // not consume them. AssertUnwindSafe: any !UnwindSafe state
            // (none here — the receiver, buffer, and service are all
            // safe to read after a panic) is documented as safe.
            let tick = std::panic::AssertUnwindSafe(audit_writer_tick(
                &mut receiver,
                &service,
                &mut buffer,
                &mut dead_letter_queue,
                &mut flush_interval,
            ))
            .catch_unwind()
            .await;
            match tick {
                Ok(()) => {
                    // Receiver dropped, writer exited normally.
                    break;
                }
                Err(panic) => {
                    tracing::error!("Audit writer panicked: {:?}", panic);
                    panic_backoff_secs = (panic_backoff_secs * 2 + 1).min(MAX_PANIC_BACKOFF_SECS);
                    tracing::warn!(
                        "Audit writer sleeping {}s before restart",
                        panic_backoff_secs
                    );
                    // Pre-commit 9: this sleep blocks shutdown. Commit 9
                    // moves it inside a select! against the shutdown
                    // channel. Sequential here to keep the panic-
                    // recovery logic reviewable on its own.
                    tokio::time::sleep(std::time::Duration::from_secs(panic_backoff_secs)).await;
                    // Continue: receiver + buffer + service are still
                    // valid. Drain any backlog the panic missed on the
                    // next iteration.
                }
            }
        }
    })
}

#[allow(clippy::too_many_arguments)]
async fn audit_writer_tick(
    receiver: &mut mpsc::Receiver<AuditEvent>,
    service: &crate::domain::audit::service::AuditService,
    buffer: &mut Vec<CreateAuditLog>,
    dead_letter_queue: &mut Vec<CreateAuditLog>,
    flush_interval: &mut tokio::time::Interval,
) {
    loop {
        tokio::select! {
            event = receiver.recv() => {
                match event {
                    Some(event) => {
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

                        if buffer.len() >= 100 {
                            if let Err(failed) = flush_buffer_with_retry(service, buffer, MAX_FLUSH_RETRIES).await {
                                dead_letter_queue.extend(failed);
                                if dead_letter_queue.len() > 1_000 {
                                    tracing::error!("Audit DLQ exceeded 1000 items, dropping oldest {} logs", dead_letter_queue.len() - 500);
                                    dead_letter_queue.drain(0..dead_letter_queue.len() - 500);
                                }
                            }
                        }
                    }
                    None => {
                        // Channel closed — flush any remaining events and exit
                        if !buffer.is_empty() {
                            if let Err(failed) = flush_buffer_with_retry(service, buffer, MAX_FLUSH_RETRIES).await {
                                dead_letter_queue.extend(failed);
                            }
                        }
                        break;
                    }
                }
            }
            _ = flush_interval.tick() => {
                if !buffer.is_empty() {
                    if let Err(failed) = flush_buffer_with_retry(service, buffer, MAX_FLUSH_RETRIES).await {
                        dead_letter_queue.extend(failed);
                    }
                }
                if !dead_letter_queue.is_empty() {
                    let mut dlq_buffer = std::mem::take(dead_letter_queue);
                    if let Err(failed) = flush_buffer_with_retry(service, &mut dlq_buffer, MAX_FLUSH_RETRIES).await {
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
