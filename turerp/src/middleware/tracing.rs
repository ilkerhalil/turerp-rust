//! Structured tracing middleware
//!
//! Logs every HTTP request/response pair with method, path, status code,
//! duration, and request ID. Uses `tracing::info!` for 2xx/3xx and
//! `tracing::error!` for 4xx/5xx.

use actix_web::body::MessageBody;
use actix_web::{dev::ServiceRequest, dev::ServiceResponse, Error, HttpMessage};
use futures::future::LocalBoxFuture;
use std::task::{Context, Poll};
use std::time::Instant;

/// Request tracing middleware
pub struct TracingMiddleware;

impl<S, B> actix_web::dev::Transform<S, ServiceRequest> for TracingMiddleware
where
    S: actix_web::dev::Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = TracingMiddlewareService<S>;
    type Future = std::future::Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        std::future::ready(Ok(TracingMiddlewareService { service }))
    }
}

/// Tracing middleware service
pub struct TracingMiddlewareService<S> {
    service: S,
}

impl<S, B> actix_web::dev::Service<ServiceRequest> for TracingMiddlewareService<S>
where
    S: actix_web::dev::Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&self, ctx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(ctx)
    }

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let method = req.method().to_string();
        let path = req.path().to_string();
        let request_id = req
            .extensions()
            .get::<String>()
            .cloned()
            .unwrap_or_default();

        let (tenant_id, user_id) = req
            .extensions()
            .get::<crate::utils::jwt::AuthClaims>()
            .map(|c| (Some(c.tenant_id), Some(c.sub.clone())))
            .unwrap_or((None, None));

        let start = Instant::now();
        let fut = self.service.call(req);

        Box::pin(async move {
            let result = fut.await;
            let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;

            match &result {
                Ok(res) => {
                    let status = res.status().as_u16();
                    if status >= 500 {
                        tracing::error!(
                            method = %method,
                            path = %path,
                            status = status,
                            duration_ms = %format!("{:.3}", elapsed_ms),
                            request_id = %request_id,
                            tenant_id = ?tenant_id,
                            user_id = ?user_id,
                            "request completed with server error"
                        );
                    } else if status >= 400 {
                        tracing::warn!(
                            method = %method,
                            path = %path,
                            status = status,
                            duration_ms = %format!("{:.3}", elapsed_ms),
                            request_id = %request_id,
                            tenant_id = ?tenant_id,
                            user_id = ?user_id,
                            "request completed with client error"
                        );
                    } else {
                        tracing::info!(
                            method = %method,
                            path = %path,
                            status = status,
                            duration_ms = %format!("{:.3}", elapsed_ms),
                            request_id = %request_id,
                            tenant_id = ?tenant_id,
                            user_id = ?user_id,
                            "request completed"
                        );
                    }
                }
                Err(e) => {
                    tracing::error!(
                        method = %method,
                        path = %path,
                        error = %e,
                        duration_ms = %format!("{:.3}", elapsed_ms),
                        request_id = %request_id,
                        tenant_id = ?tenant_id,
                        user_id = ?user_id,
                        "request failed"
                    );
                }
            }

            result
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, web, App, HttpResponse};

    #[actix_web::test]
    async fn test_tracing_middleware_ok() {
        let app = test::init_service(
            App::new()
                .wrap(TracingMiddleware)
                .route("/", web::get().to(|| async { HttpResponse::Ok().finish() })),
        )
        .await;

        let req = test::TestRequest::default().to_request();
        let res = test::call_service(&app, req).await;
        assert!(res.status().is_success());
    }

    #[actix_web::test]
    async fn test_tracing_middleware_error() {
        let app = test::init_service(App::new().wrap(TracingMiddleware).route(
            "/",
            web::get().to(|| async { HttpResponse::InternalServerError().finish() }),
        ))
        .await;

        let req = test::TestRequest::default().to_request();
        let res = test::call_service(&app, req).await;
        assert!(res.status().is_server_error());
    }
}
