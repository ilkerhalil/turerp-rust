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

/// Global gate middleware: applies multiple (path_prefix, flag_name) rules
/// at the App level. For each request, finds the longest matching prefix
/// and, if one is found, checks the flag. If the flag is disabled, the
/// request is short-circuited with 404. If no rule matches the path, the
/// request is forwarded unchanged (non-gated route).
///
/// This is the recommended way to wire gates from `main.rs` because it
/// doesn't require wrapping each module's `configure` callback in a
/// `web::scope` (which would change the route paths). The gate is
/// registered once via `.wrap(GlobalGate::new(rules, service))` on the
/// `App` and the rules list is the single source of truth.
pub struct GlobalGate {
    /// (path_prefix, flag_name) pairs. Ordered by registration; longest
    /// prefix wins when multiple rules match the same path.
    rules: Vec<(String, String)>,
    /// Feature-flag service used to evaluate each rule. Must already be
    /// registered as `web::Data<FeatureFlagService>` in the app.
    feature_service: actix_web::web::Data<FeatureFlagService>,
}

impl GlobalGate {
    pub fn new(
        rules: Vec<(String, String)>,
        feature_service: actix_web::web::Data<FeatureFlagService>,
    ) -> Self {
        Self {
            rules,
            feature_service,
        }
    }
}

impl<S, B> actix_web::dev::Transform<S, actix_web::dev::ServiceRequest> for GlobalGate
where
    S: actix_web::dev::Service<
            actix_web::dev::ServiceRequest,
            Response = actix_web::dev::ServiceResponse<B>,
            Error = actix_web::Error,
        > + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = actix_web::dev::ServiceResponse<actix_web::body::EitherBody<B>>;
    type Error = actix_web::Error;
    type InitError = ();
    type Transform = GlobalGateMiddleware<S>;
    type Future = std::future::Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        std::future::ready(Ok(GlobalGateMiddleware {
            service: std::rc::Rc::new(service),
            rules: self.rules.clone(),
            feature_service: self.feature_service.clone(),
        }))
    }
}

pub struct GlobalGateMiddleware<S> {
    service: std::rc::Rc<S>,
    rules: Vec<(String, String)>,
    feature_service: actix_web::web::Data<FeatureFlagService>,
}

impl<S, B> actix_web::dev::Service<actix_web::dev::ServiceRequest> for GlobalGateMiddleware<S>
where
    S: actix_web::dev::Service<
            actix_web::dev::ServiceRequest,
            Response = actix_web::dev::ServiceResponse<B>,
            Error = actix_web::Error,
        > + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = actix_web::dev::ServiceResponse<actix_web::body::EitherBody<B>>;
    type Error = actix_web::Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    actix_web::dev::forward_ready!(service);

    fn call(&self, req: actix_web::dev::ServiceRequest) -> Self::Future {
        let svc = self.service.clone();
        let rules = self.rules.clone();
        let feature_service = self.feature_service.clone();
        let path = req.path().to_string();

        Box::pin(async move {
            // Find longest matching prefix.
            let matched_flag = rules
                .iter()
                .filter(|(prefix, _)| path.starts_with(prefix.as_str()))
                .max_by_key(|(prefix, _)| prefix.len())
                .map(|(_, flag)| flag.clone());

            let Some(flag) = matched_flag else {
                // No rule matches → forward as-is (non-gated route).
                let res = svc.call(req).await?;
                return Ok(res.map_into_left_body());
            };

            // Extract tenant_id from AuthClaims (set by AuthUser middleware).
            // The gate must run AFTER JwtAuthMiddleware, which is registered
            // as a wrap above this one in main.rs.
            let tenant_id = req.extensions().get::<AuthClaims>().map(|c| c.tenant_id);

            let tenant_id = match tenant_id {
                Some(t) => t,
                None => {
                    tracing::debug!(
                        flag = %flag,
                        path = %path,
                        "global_gate: no AuthClaims in extensions, fail-closed 404"
                    );
                    let resp = actix_web::HttpResponse::NotFound()
                        .json(serde_json::json!({"error": "Not found"}));
                    let (req_parts, _) = req.into_parts();
                    return Ok(
                        actix_web::dev::ServiceResponse::new(req_parts, resp).map_into_right_body()
                    );
                }
            };

            let enabled = match feature_service.is_enabled(&flag, Some(tenant_id)).await {
                Ok(b) => b,
                Err(e) => {
                    tracing::warn!(
                        flag = %flag,
                        path = %path,
                        tenant_id = tenant_id,
                        error = %e,
                        "global_gate: is_enabled() failed, failing open"
                    );
                    true
                }
            };

            if !enabled {
                tracing::debug!(
                    flag = %flag,
                    path = %path,
                    tenant_id = tenant_id,
                    "global_gate: flag disabled, returning 404"
                );
                let resp = actix_web::HttpResponse::NotFound()
                    .json(serde_json::json!({"error": "Not found"}));
                let (req_parts, _) = req.into_parts();
                return Ok(
                    actix_web::dev::ServiceResponse::new(req_parts, resp).map_into_right_body()
                );
            }

            let res = svc.call(req).await?;
            Ok(res.map_into_left_body())
        })
    }
}
