//! Tests for production-readiness hardening added in 2026-06-05.
//!
//! These tests cover two distinct surfaces:
//! 1. `is_sensitive_audit_event` — the heuristic that decides whether
//!    an audit event should block on the channel instead of being
//!    dropped on overflow. Wrong classification = either silent
//!    security-incident-loss or a request-path stall under load.
//! 2. (placeholder for future tests as the PR is extended)

use turerp::middleware::audit::{is_sensitive_audit_event, AuditEvent};

fn event(action: &str, path: &str, status_code: i16) -> AuditEvent {
    AuditEvent {
        tenant_id: 1,
        user_id: 7,
        username: "alice".into(),
        action: action.into(),
        path: path.into(),
        status_code,
        request_id: "req-1".into(),
        ip_address: Some("127.0.0.1".into()),
        user_agent: Some("test/1.0".into()),
    }
}

#[test]
fn server_error_5xx_is_sensitive() {
    assert!(is_sensitive_audit_event(&event("GET", "/api/v1/foo", 500)));
    assert!(is_sensitive_audit_event(&event("POST", "/api/v1/foo", 503)));
    assert!(is_sensitive_audit_event(&event("PUT", "/api/v1/foo", 502)));
}

#[test]
fn auth_keywords_in_path_are_sensitive() {
    assert!(is_sensitive_audit_event(&event(
        "POST",
        "/api/v1/auth/login",
        200
    )));
    assert!(is_sensitive_audit_event(&event(
        "POST",
        "/api/v1/auth/logout",
        200
    )));
    assert!(is_sensitive_audit_event(&event(
        "POST",
        "/api/v1/mfa/verify",
        200
    )));
}

#[test]
fn auth_keywords_in_action_are_sensitive() {
    assert!(is_sensitive_audit_event(&event(
        "auth.refresh",
        "/api/v1/foo",
        200
    )));
    assert!(is_sensitive_audit_event(&event("POST", "/api/v1/foo", 200)).eq(&false));
    assert!(is_sensitive_audit_event(&event(
        "role.assign",
        "/api/v1/foo",
        200
    )));
    assert!(is_sensitive_audit_event(&event(
        "permission.grant",
        "/api/v1/foo",
        200
    )));
}

#[test]
fn routine_2xx_requests_are_not_sensitive() {
    assert!(!is_sensitive_audit_event(&event(
        "GET",
        "/api/v1/products",
        200
    )));
    assert!(!is_sensitive_audit_event(&event(
        "POST",
        "/api/v1/invoices",
        201
    )));
    assert!(!is_sensitive_audit_event(&event(
        "DELETE",
        "/api/v1/cache",
        204
    )));
}

#[test]
fn routine_client_4xx_errors_are_not_sensitive() {
    // 4xx on non-auth paths is "client made a bad request"; the
    // request is already in tracing, no need to block the request
    // path waiting for a DB write.
    assert!(!is_sensitive_audit_event(&event(
        "GET",
        "/api/v1/missing",
        404
    )));
    assert!(!is_sensitive_audit_event(&event(
        "POST",
        "/api/v1/users",
        400
    )));
    // 4xx on auth paths IS sensitive — failed logins are exactly
    // the signal we cannot afford to lose.
    assert!(is_sensitive_audit_event(&event(
        "POST",
        "/api/v1/auth/login",
        401
    )));
}
