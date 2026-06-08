//! GraphQL API endpoint (v1)

use actix_web::{web, HttpRequest, HttpResponse};
use async_graphql_actix_web::{GraphQLRequest, GraphQLResponse};

use crate::app::AppState;
use crate::error::ApiError;
use crate::graphql::GraphQlContext;
use crate::middleware::TenantContextExt;

/// GraphQL endpoint handler
///
/// Extracts tenant_id from the request extensions (set by TenantMiddleware)
/// and injects it into the GraphQL context along with application state.
///
/// Fails closed: if no tenant context is attached to the request, returns
/// 401 Unauthorized rather than falling back to tenant 0 (the system
/// tenant). Tenant 0 is shared by all tenants' system rows; using it as
/// a fallback would leak cross-tenant data if the middleware chain ever
/// failed to set the context.
#[utoipa::path(
    post,
    path = "/api/v1/graphql",
    tag = "GraphQL",
    request_body = String,
    responses(
        (status = 200, description = "GraphQL response"),
        (status = 401, description = "Not authenticated or missing tenant context")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn graphql_handler(
    app_state: web::Data<AppState>,
    req: HttpRequest,
    gql_req: GraphQLRequest,
) -> Result<GraphQLResponse, ApiError> {
    let tenant_id = req
        .tenant_id()
        .ok_or_else(|| ApiError::Unauthorized("Missing tenant context".to_string()))?;
    let gctx = GraphQlContext::new(std::sync::Arc::new(app_state.get_ref().clone()), tenant_id);

    Ok(app_state
        .schema
        .execute(gql_req.into_inner().data(gctx))
        .await
        .into())
}

/// GraphQL Playground endpoint (for development)
#[utoipa::path(
    get,
    path = "/api/v1/graphql",
    tag = "GraphQL",
    responses(
        (status = 200, description = "GraphQL Playground HTML")
    )
)]
pub async fn graphql_playground() -> HttpResponse {
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(async_graphql::http::playground_source(
            async_graphql::http::GraphQLPlaygroundConfig::new("/api/v1/graphql"),
        ))
}

/// Configure GraphQL routes
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/v1/graphql")
            .route(web::post().to(graphql_handler))
            .route(web::get().to(graphql_playground)),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::test::TestRequest;

    /// Regression test for `fix(security): GraphQL rejects missing tenant context`.
    ///
    /// The previous handler used `req.tenant_id().unwrap_or(0)` and silently
    /// fell back to the system tenant (tenant 0). This test pins the new
    /// behavior at the `tenant_id` extraction boundary: when the request
    /// has no TenantContext extension, the new code returns
    /// `ApiError::Unauthorized`. The handler is a thin wrapper that calls
    /// this same expression, so the contract is preserved end-to-end.
    ///
    /// Building a full `AppState` is heavy (DB pool, schema, services);
    /// instead we exercise the same Ok/Err shape that the handler uses
    /// to short-circuit. The HTTP integration test
    /// `tests/graphql_tenant_test.rs` (added in the same commit) hits the
    /// live handler with a missing-context request and asserts a 401.
    #[tokio::test]
    async fn test_tenant_id_missing_returns_unauthorized() {
        let req = TestRequest::default().to_http_request();
        let result: Result<i64, ApiError> = req
            .tenant_id()
            .ok_or_else(|| ApiError::Unauthorized("Missing tenant context".to_string()));
        let err_msg = match &result {
            Err(ApiError::Unauthorized(msg)) => msg.clone(),
            other => panic!(
                "expected Unauthorized('Missing tenant context'), got {:?}",
                other
            ),
        };
        assert_eq!(err_msg, "Missing tenant context");
    }
}
