//! Auth repository traits and implementations

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::Arc;

use crate::error::ApiError;

/// Trait for revoked token storage backends
#[async_trait]
pub trait RevokedTokenStore: Send + Sync {
    /// Check if a token is revoked.
    ///
    /// Returns `Ok(true)` if the token is revoked, `Ok(false)` if not.
    /// Returns `Err` if the underlying store cannot be reached (e.g. DB
    /// connection error). Callers MUST treat `Err` as "revoked" (deny the
    /// request) to fail closed — see issue #324.
    async fn is_revoked(&self, token_hash: &str) -> Result<bool, ApiError>;
    async fn revoke(&self, token_hash: &str, _expires_at: DateTime<Utc>) -> Result<(), ApiError>;

    /// Delete all expired revoked-token entries (where `expires_at < now`).
    /// Returns the number of rows removed. The default implementation is a
    /// no-op, suitable for stores that don't track expiry.
    /// Production backends (PostgreSQL) and the in-memory store override
    /// this to prevent unbounded growth — see issues #329 and #346.
    async fn purge_expired(&self) -> Result<u64, ApiError> {
        Ok(0)
    }
}

/// In-memory revoked token store (for development / single-instance deployment).
///
/// Tracks each revoked token hash alongside its JWT expiry timestamp so that
/// `purge_expired` can remove entries once the underlying JWT has expired
/// naturally (issue #346). Without this, the internal set grows without
/// bound as users log out / refresh tokens.
pub struct InMemoryRevokedTokenStore {
    revoked: Arc<Mutex<HashMap<String, DateTime<Utc>>>>,
}

impl Default for InMemoryRevokedTokenStore {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryRevokedTokenStore {
    pub fn new() -> Self {
        Self {
            revoked: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl RevokedTokenStore for InMemoryRevokedTokenStore {
    async fn is_revoked(&self, token_hash: &str) -> Result<bool, ApiError> {
        let mut revoked = self.revoked.lock();
        // Inline expiry check: if the entry exists but its JWT has already
        // expired naturally, treat it as not-revoked and remove it to keep
        // the set from growing.
        if let Some(expires_at) = revoked.get(token_hash) {
            if *expires_at < Utc::now() {
                revoked.remove(token_hash);
                return Ok(false);
            }
            return Ok(true);
        }
        Ok(false)
    }

    async fn revoke(&self, token_hash: &str, expires_at: DateTime<Utc>) -> Result<(), ApiError> {
        self.revoked
            .lock()
            .insert(token_hash.to_string(), expires_at);
        Ok(())
    }

    async fn purge_expired(&self) -> Result<u64, ApiError> {
        let now = Utc::now();
        let mut revoked = self.revoked.lock();
        let before = revoked.len();
        revoked.retain(|_, expires_at| *expires_at >= now);
        let removed = before - revoked.len();
        Ok(removed as u64)
    }
}

pub type BoxRevokedTokenStore = Arc<dyn RevokedTokenStore>;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_revoke_and_is_revoked() {
        let store = InMemoryRevokedTokenStore::new();
        let future = Utc::now() + chrono::Duration::hours(1);

        assert!(!store.is_revoked("abc").await.unwrap());
        store.revoke("abc", future).await.unwrap();
        assert!(store.is_revoked("abc").await.unwrap());
    }

    #[tokio::test]
    async fn test_expired_token_not_revoked_and_evicted() {
        let store = InMemoryRevokedTokenStore::new();
        let past = Utc::now() - chrono::Duration::hours(1);

        store.revoke("abc", past).await.unwrap();
        // The token's JWT has already expired, so it should be treated as
        // not-revoked and evicted from the store inline.
        assert!(!store.is_revoked("abc").await.unwrap());
        // Second call confirms the entry was removed.
        assert!(store.revoked.lock().is_empty());
    }

    #[tokio::test]
    async fn test_purge_expired_removes_only_expired() {
        let store = InMemoryRevokedTokenStore::new();
        let past = Utc::now() - chrono::Duration::hours(1);
        let future = Utc::now() + chrono::Duration::hours(1);

        store.revoke("expired1", past).await.unwrap();
        store.revoke("expired2", past).await.unwrap();
        store.revoke("alive1", future).await.unwrap();
        store.revoke("alive2", future).await.unwrap();

        let removed = store.purge_expired().await.unwrap();
        assert_eq!(removed, 2);
        assert_eq!(store.revoked.lock().len(), 2);
        assert!(!store.is_revoked("expired1").await.unwrap());
        assert!(!store.is_revoked("expired2").await.unwrap());
        assert!(store.is_revoked("alive1").await.unwrap());
        assert!(store.is_revoked("alive2").await.unwrap());
    }

    #[tokio::test]
    async fn test_purge_expired_empty_store() {
        let store = InMemoryRevokedTokenStore::new();
        let removed = store.purge_expired().await.unwrap();
        assert_eq!(removed, 0);
    }
}
