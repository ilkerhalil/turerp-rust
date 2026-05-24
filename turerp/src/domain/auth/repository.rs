//! Auth repository traits and implementations

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use parking_lot::Mutex;
use std::collections::HashSet;
use std::sync::Arc;

use crate::error::ApiError;

/// Trait for revoked token storage backends
#[async_trait]
pub trait RevokedTokenStore: Send + Sync {
    async fn is_revoked(&self, token_hash: &str) -> bool;
    async fn revoke(&self, token_hash: &str, _expires_at: DateTime<Utc>) -> Result<(), ApiError>;
}

/// In-memory revoked token store (for development / single-instance deployment)
pub struct InMemoryRevokedTokenStore {
    revoked: Arc<Mutex<HashSet<String>>>,
}

impl Default for InMemoryRevokedTokenStore {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryRevokedTokenStore {
    pub fn new() -> Self {
        Self {
            revoked: Arc::new(Mutex::new(HashSet::new())),
        }
    }
}

#[async_trait]
impl RevokedTokenStore for InMemoryRevokedTokenStore {
    async fn is_revoked(&self, token_hash: &str) -> bool {
        self.revoked.lock().contains(token_hash)
    }

    async fn revoke(&self, token_hash: &str, _expires_at: DateTime<Utc>) -> Result<(), ApiError> {
        self.revoked.lock().insert(token_hash.to_string());
        Ok(())
    }
}

pub type BoxRevokedTokenStore = Arc<dyn RevokedTokenStore>;
