//! GraphQL API endpoint (v1)

use actix_web::{web, HttpRequest, HttpResponse};
use async_graphql_actix_web::{GraphQLRequest, GraphQLResponse};

use crate::app::AppState;
use crate::graphql::GraphQlContext;
use crate::middleware::TenantContextExt;

/// GraphQL endpoint handler
///
/// Extracts tenant_id from the request extensions (set by TenantMiddleware)
/// and injects it into the GraphQL context along with application state.
#[utoipa::path(
    post,
    path = "/api/v1/graphql",
    tag = "GraphQL",
    request_body = String,
    responses(
        (status = 200, description = "GraphQL response"),
        (status = 401, description = "Not authenticated")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn graphql_handler(
    app_state: web::Data<AppState>,
    req: HttpRequest,
    gql_req: GraphQLRequest,
) -> GraphQLResponse {
    let tenant_id = req.tenant_id().unwrap_or(0);
    let gctx = GraphQlContext::new(std::sync::Arc::new(app_state.get_ref().clone()), tenant_id);

    app_state
        .schema
        .execute(gql_req.into_inner().data(gctx))
        .await
        .into()
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
