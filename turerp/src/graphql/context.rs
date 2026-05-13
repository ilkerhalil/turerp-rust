//! GraphQL context for tenant isolation and service access

use crate::app::AppState;
use std::sync::Arc;

/// GraphQL request context containing application state and tenant isolation
#[derive(Clone)]
pub struct GraphQlContext {
    pub app_state: Arc<AppState>,
    pub tenant_id: i64,
}

impl GraphQlContext {
    pub fn new(app_state: Arc<AppState>, tenant_id: i64) -> Self {
        Self {
            app_state,
            tenant_id,
        }
    }
}
