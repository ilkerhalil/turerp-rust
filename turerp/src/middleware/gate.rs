//! Feature-flag gate middleware.
//!
//! Wraps a route scope to reject requests with `404 Not Found` when the
//! configured feature flag is disabled for the caller's tenant. When the
//! flag is enabled, the request is forwarded unchanged.
//!
//! Behavior:
//! - Reads `AuthClaims` from request extensions (set by the upstream
//!   AuthUser middleware — must run BEFORE this gate).
//! - Calls `FeatureFlagService::is_enabled(flag_name, Some(tenant_id))`.
//! - On `true`: forward. On `false`: short-circuit 404.
//! - On missing AuthClaims (route is public): 404 (fail-closed).
//! - On service error: log WARN, **fail-open** (forward). Rationale:
//!   the gate must never break a route that's working; flag infra
//!   outages should be visible but not user-facing. This is a deliberate
//!   trade-off documented in RUNBOOK.md.

use std::future::{ready, Ready};
use std::rc::Rc;

use actix_web::body::EitherBody;
use actix_web::dev::{Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::{Error, HttpMessage, HttpResponse};
use futures::future::LocalBoxFuture;
use serde_json::json;

use crate::domain::feature::FeatureFlagService;
use crate::utils::jwt::AuthClaims;

/// Configuration for the gate middleware.
#[derive(Clone, Debug)]
pub struct GateConfig {
    /// Feature flag name to check (e.g. "tier2.manufacturing").
    pub flag: String,
}

/// Gate middleware factory.
pub struct FeatureFlagGate {
    config: GateConfig,
}

impl FeatureFlagGate {
    pub fn new(config: GateConfig) -> Self {
        Self { config }
    }
}

impl<S, B> Transform<S, ServiceRequest> for FeatureFlagGate
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type InitError = ();
    type Transform = FeatureFlagGateMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(FeatureFlagGateMiddleware {
            service: Rc::new(service),
            config: self.config.clone(),
        }))
    }
}

/// Sugar: `gate(GateConfig { flag: "..." })` — equivalent to
/// `actix_web::middleware::from_fn`-style wrapping for clarity at the
/// call site.
pub fn gate(config: GateConfig) -> FeatureFlagGate {
    FeatureFlagGate::new(config)
}

pub struct FeatureFlagGateMiddleware<S> {
    service: Rc<S>,
    config: GateConfig,
}

impl<S, B> Service<ServiceRequest> for FeatureFlagGateMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    actix_web::dev::forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let svc = self.service.clone();
        let cfg = self.config.clone();

        Box::pin(async move {
            // Extract tenant_id from AuthClaims (set by AuthUser middleware).
            let tenant_id = req.extensions().get::<AuthClaims>().map(|c| c.tenant_id);

            let tenant_id = match tenant_id {
                Some(t) => t,
                None => {
                    tracing::debug!(
                        flag = %cfg.flag,
                        "gate: no AuthClaims in extensions, fail-closed 404"
                    );
                    let resp = HttpResponse::NotFound().json(json!({"error": "Not found"}));
                    let (req_parts, _) = req.into_parts();
                    return Ok(ServiceResponse::new(req_parts, resp).map_into_right_body());
                }
            };

            // Fetch the flag service from app_data.
            let service = match req.app_data::<actix_web::web::Data<FeatureFlagService>>() {
                Some(s) => s.clone(),
                None => {
                    tracing::error!(
                        "gate: FeatureFlagService not in app_data — \
                         misconfiguration, fail-closed 404"
                    );
                    let resp = HttpResponse::NotFound().json(json!({"error": "Not found"}));
                    let (req_parts, _) = req.into_parts();
                    return Ok(ServiceResponse::new(req_parts, resp).map_into_right_body());
                }
            };

            let enabled = match service.is_enabled(&cfg.flag, Some(tenant_id)).await {
                Ok(b) => b,
                Err(e) => {
                    tracing::warn!(
                        flag = %cfg.flag,
                        tenant_id = tenant_id,
                        error = %e,
                        "gate: is_enabled() failed, failing open"
                    );
                    true
                }
            };

            if !enabled {
                tracing::debug!(
                    flag = %cfg.flag,
                    tenant_id = tenant_id,
                    "gate: flag disabled, returning 404"
                );
                let resp = HttpResponse::NotFound().json(json!({"error": "Not found"}));
                let (req_parts, _) = req.into_parts();
                return Ok(ServiceResponse::new(req_parts, resp).map_into_right_body());
            }

            let res = svc.call(req).await?;
            Ok(res.map_into_left_body())
        })
    }
}

/// Wrap a v1 module's `configure` so all its routes are gated by `cfg.flag`.
/// When the flag is off, the gate returns 404 and the inner configure
/// still registers its routes (the gate intercepts every request).
///
/// The `web::scope("")` is a no-op prefix but is required so we can
/// apply `.wrap(gate_mw)` around the inner `configure` without
/// disturbing the inner module's route registration paths.
pub fn gate_v1<F>(
    cfg: GateConfig,
    inner: F,
) -> impl FnOnce(&mut actix_web::web::ServiceConfig) + Clone
where
    F: FnOnce(&mut actix_web::web::ServiceConfig) + Clone,
{
    move |svc| {
        let gate_mw = gate(cfg);
        svc.service(actix_web::web::scope("").wrap(gate_mw).configure(inner));
    }
}
