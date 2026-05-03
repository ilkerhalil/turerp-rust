//! Request ID middleware for distributed tracing
//!
//! Generates a unique request ID for each request and adds it as a response header

use actix_web::body::MessageBody;
use actix_web::http::header::{HeaderName, HeaderValue};
use actix_web::{dev::ServiceRequest, dev::ServiceResponse, Error, HttpMessage};
use futures::future::LocalBoxFuture;
use std::task::{Context, Poll};
use uuid::Uuid;

/// Request ID middleware
pub struct RequestIdMiddleware;

impl<S, B> actix_web::dev::Transform<S, ServiceRequest> for RequestIdMiddleware
where
    S: actix_web::dev::Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: MessageBody + 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = RequestIdMiddlewareService<S>;
    type Future = std::future::Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        std::future::ready(Ok(RequestIdMiddlewareService { service }))
    }
}

/// Request ID middleware service
pub struct RequestIdMiddlewareService<S> {
    service: S,
}

impl<S, B> actix_web::dev::Service<ServiceRequest> for RequestIdMiddlewareService<S>
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
        // Use existing X-Request-ID header if present, otherwise generate new
        let request_id = req
            .headers()
            .get("X-Request-ID")
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string())
            .unwrap_or_else(|| Uuid::new_v4().to_string());

        // Store in request extensions for downstream handlers
        req.extensions_mut().insert(request_id.clone());

        let fut = self.service.call(req);
        Box::pin(async move {
            let mut res = fut.await?;
            res.headers_mut().insert(
                HeaderName::from_static("x-request-id"),
                HeaderValue::from_str(&request_id)
                    .unwrap_or_else(|_| HeaderValue::from_static("unknown")),
            );
            Ok(res)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, web, App, HttpResponse};

    #[actix_web::test]
    async fn test_request_id_added() {
        let app = test::init_service(
            App::new()
                .wrap(RequestIdMiddleware)
                .route("/", web::get().to(|| async { HttpResponse::Ok().finish() })),
        )
        .await;

        let req = test::TestRequest::default().to_request();
        let res = test::call_service(&app, req).await;

        // Response should have X-Request-ID header
        assert!(res.headers().contains_key("x-request-id"));
    }

    #[actix_web::test]
    async fn test_existing_request_id_preserved() {
        let app = test::init_service(
            App::new()
                .wrap(RequestIdMiddleware)
                .route("/", web::get().to(|| async { HttpResponse::Ok().finish() })),
        )
        .await;

        let req = test::TestRequest::default()
            .insert_header(("X-Request-ID", "test-id-123"))
            .to_request();
        let res = test::call_service(&app, req).await;

        // Should preserve existing request ID
        let request_id = res.headers().get("x-request-id").unwrap();
        assert_eq!(request_id.to_str().unwrap(), "test-id-123");
    }
}
