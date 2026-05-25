//! GraphQL API module
//!
//! Provides a unified async-graphql schema for querying ERP entities
//! with tenant isolation, pagination, and filtering.

pub mod context;
pub mod query;
pub mod types;

use async_graphql::{EmptyMutation, EmptySubscription, Schema};

pub use context::GraphQlContext;
pub use query::QueryRoot;
pub use types::*;

/// The GraphQL application schema type
pub type AppSchema = Schema<QueryRoot, EmptyMutation, EmptySubscription>;

/// Create a new GraphQL schema
pub fn create_schema(enable_introspection: bool) -> AppSchema {
    let mut builder = Schema::build(QueryRoot, EmptyMutation, EmptySubscription);
    if !enable_introspection {
        builder = builder.disable_introspection();
    }
    builder.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::create_app_state_in_memory;
    use crate::config::Config;
    use async_graphql::InputType;
    use std::sync::Arc;

    fn test_schema() -> AppSchema {
        create_schema(true)
    }

    async fn test_context(tenant_id: i64) -> GraphQlContext {
        let config = Config {
            encryption_key: "YWJjZGVmZ2hpamtsbW5vcHFyc3R1dnd4eXoxMjM0NTY=".to_string(),
            ..Config::default()
        };
        let state = create_app_state_in_memory(&config)
            .await
            .expect("app state creation failed");
        GraphQlContext::new(Arc::new(state), tenant_id)
    }

    #[tokio::test]
    async fn test_users_query_empty() {
        let schema = test_schema();
        let gctx = test_context(1).await;
        let res = schema
            .execute(
                async_graphql::Request::new("{ users(page: 1, perPage: 10) { items { id username email } pageInfo { total } } }")
                    .data(gctx),
            )
            .await;
        assert!(res.errors.is_empty(), "errors: {:?}", res.errors);
        let data = res.data.into_json().unwrap();
        let items = data["users"]["items"].as_array().unwrap();
        assert!(items.is_empty());
    }

    #[tokio::test]
    async fn test_employees_query_empty() {
        let schema = test_schema();
        let gctx = test_context(1).await;
        let res = schema
            .execute(
                async_graphql::Request::new("{ employees(page: 1, perPage: 10) { items { id employeeNumber firstName lastName } pageInfo { total } } }")
                    .data(gctx),
            )
            .await;
        assert!(res.errors.is_empty(), "errors: {:?}", res.errors);
        let data = res.data.into_json().unwrap();
        let items = data["employees"]["items"].as_array().unwrap();
        assert!(items.is_empty());
    }

    #[tokio::test]
    async fn test_invoices_query_empty() {
        let schema = test_schema();
        let gctx = test_context(1).await;
        let res = schema
            .execute(
                async_graphql::Request::new("{ invoices(page: 1, perPage: 10) { items { id invoiceNumber status totalAmount } pageInfo { total } } }")
                    .data(gctx),
            )
            .await;
        assert!(res.errors.is_empty(), "errors: {:?}", res.errors);
        let data = res.data.into_json().unwrap();
        let items = data["invoices"]["items"].as_array().unwrap();
        assert!(items.is_empty());
    }

    #[tokio::test]
    async fn test_products_query_empty() {
        let schema = test_schema();
        let gctx = test_context(1).await;
        let res = schema
            .execute(
                async_graphql::Request::new("{ products(page: 1, perPage: 10) { items { id code name salePrice } pageInfo { total } } }")
                    .data(gctx),
            )
            .await;
        assert!(res.errors.is_empty(), "errors: {:?}", res.errors);
        let data = res.data.into_json().unwrap();
        let items = data["products"]["items"].as_array().unwrap();
        assert!(items.is_empty());
    }

    #[tokio::test]
    async fn test_caris_query_empty() {
        let schema = test_schema();
        let gctx = test_context(1).await;
        let res = schema
            .execute(
                async_graphql::Request::new("{ caris(page: 1, perPage: 10) { items { id code name cariType status } pageInfo { total } } }")
                    .data(gctx),
            )
            .await;
        assert!(res.errors.is_empty(), "errors: {:?}", res.errors);
        let data = res.data.into_json().unwrap();
        let items = data["caris"]["items"].as_array().unwrap();
        assert!(items.is_empty());
    }

    #[tokio::test]
    async fn test_graphql_role_enum() {
        let role = GraphQlRole::Admin;
        assert_eq!(
            role.to_value(),
            async_graphql::Value::String("ADMIN".into())
        );
    }

    #[tokio::test]
    async fn test_graphql_cari_type_enum() {
        let ct = GraphQlCariType::Both;
        assert_eq!(ct.to_value(), async_graphql::Value::String("BOTH".into()));
    }

    #[tokio::test]
    async fn test_graphql_invoice_status_enum() {
        let s = GraphQlInvoiceStatus::PartiallyPaid;
        assert_eq!(
            s.to_value(),
            async_graphql::Value::String("PARTIALLY_PAID".into())
        );
    }

    #[tokio::test]
    async fn test_tenant_isolation_rejected_without_context() {
        let schema = test_schema();
        let res = schema
            .execute("{ users(page: 1, perPage: 10) { items { id } } }")
            .await;
        assert!(!res.errors.is_empty(), "expected error without context");
    }

    #[tokio::test]
    async fn test_introspection_enabled() {
        let schema = create_schema(true);
        let res = schema.execute("{ __schema { types { name } } }").await;
        assert!(
            res.errors.is_empty(),
            "introspection should be enabled: {:?}",
            res.errors
        );
    }

    #[tokio::test]
    async fn test_introspection_disabled() {
        let schema = create_schema(false);
        let res = schema.execute("{ __schema { types { name } } }").await;
        // When introspection is disabled, async-graphql may either return an error
        // or omit the introspection fields from the result. We accept either signal.
        let errors = res.errors.clone();
        let data_json = res.data.into_json().unwrap();
        let schema_null = data_json
            .get("__schema")
            .map(|v| v.is_null())
            .unwrap_or(true);
        assert!(
            !errors.is_empty() || schema_null,
            "introspection should be disabled: errors={:?}, data={}",
            errors,
            data_json
        );
    }
}
