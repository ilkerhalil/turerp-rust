//! Blockchain ledger repository trait and in-memory implementation

use async_trait::async_trait;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;

use crate::domain::edefter::blockchain::model::{HashChainEntry, LedgerHashState, MerkleTree};
use crate::error::ApiError;

/// Repository trait for blockchain ledger operations
#[async_trait]
pub trait BlockchainLedgerRepository: Send + Sync {
    /// Create a new hash chain entry
    async fn create_hash_entry(&self, entry: HashChainEntry) -> Result<HashChainEntry, ApiError>;

    /// Find all hash chain entries for a period
    async fn find_hash_entries(
        &self,
        period_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<HashChainEntry>, ApiError>;

    /// Find the last hash chain entry for a period
    async fn find_last_hash_entry(
        &self,
        period_id: i64,
        tenant_id: i64,
    ) -> Result<Option<HashChainEntry>, ApiError>;

    /// Save a Merkle tree
    async fn save_merkle_tree(&self, tree: MerkleTree) -> Result<MerkleTree, ApiError>;

    /// Find Merkle tree for a period
    async fn find_merkle_tree(
        &self,
        period_id: i64,
        tenant_id: i64,
    ) -> Result<Option<MerkleTree>, ApiError>;

    /// Save ledger hash state
    async fn save_ledger_hash_state(
        &self,
        state: LedgerHashState,
    ) -> Result<LedgerHashState, ApiError>;

    /// Find ledger hash state for a period
    async fn find_ledger_hash_state(
        &self,
        period_id: i64,
        tenant_id: i64,
    ) -> Result<Option<LedgerHashState>, ApiError>;
}

/// Type alias for boxed blockchain ledger repository
pub type BoxBlockchainLedgerRepository = Arc<dyn BlockchainLedgerRepository>;

struct Inner {
    hash_entries: HashMap<i64, HashChainEntry>,
    period_hash_entries: HashMap<i64, Vec<i64>>,
    merkle_trees: HashMap<i64, MerkleTree>,
    ledger_hash_states: HashMap<i64, LedgerHashState>,
    next_id: AtomicI64,
}

/// In-memory blockchain ledger repository for testing and development
pub struct InMemoryBlockchainLedgerRepository {
    inner: Mutex<Inner>,
}

impl InMemoryBlockchainLedgerRepository {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(Inner {
                hash_entries: HashMap::new(),
                period_hash_entries: HashMap::new(),
                merkle_trees: HashMap::new(),
                ledger_hash_states: HashMap::new(),
                next_id: AtomicI64::new(1),
            }),
        }
    }
}

impl Default for InMemoryBlockchainLedgerRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl BlockchainLedgerRepository for InMemoryBlockchainLedgerRepository {
    async fn create_hash_entry(&self, entry: HashChainEntry) -> Result<HashChainEntry, ApiError> {
        let mut inner = self.inner.lock();
        let id = inner.next_id.fetch_add(1, Ordering::SeqCst);

        let stored = HashChainEntry {
            id,
            tenant_id: entry.tenant_id,
            period_id: entry.period_id,
            entry_id: entry.entry_id,
            previous_hash: entry.previous_hash,
            entry_hash: entry.entry_hash,
            created_at: entry.created_at,
        };

        inner.hash_entries.insert(id, stored.clone());
        inner
            .period_hash_entries
            .entry(entry.period_id)
            .or_default()
            .push(id);

        Ok(stored)
    }

    async fn find_hash_entries(
        &self,
        period_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<HashChainEntry>, ApiError> {
        let inner = self.inner.lock();
        let ids = inner
            .period_hash_entries
            .get(&period_id)
            .cloned()
            .unwrap_or_default();
        let mut results: Vec<_> = ids
            .into_iter()
            .filter_map(|id| inner.hash_entries.get(&id).cloned())
            .filter(|e| e.tenant_id == tenant_id)
            .collect();
        results.sort_by_key(|e| e.id);
        Ok(results)
    }

    async fn find_last_hash_entry(
        &self,
        period_id: i64,
        tenant_id: i64,
    ) -> Result<Option<HashChainEntry>, ApiError> {
        let entries = self.find_hash_entries(period_id, tenant_id).await?;
        Ok(entries.into_iter().last())
    }

    async fn save_merkle_tree(&self, tree: MerkleTree) -> Result<MerkleTree, ApiError> {
        let mut inner = self.inner.lock();
        inner.merkle_trees.insert(tree.period_id, tree.clone());
        Ok(tree)
    }

    async fn find_merkle_tree(
        &self,
        period_id: i64,
        tenant_id: i64,
    ) -> Result<Option<MerkleTree>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .merkle_trees
            .get(&period_id)
            .filter(|t| t.tenant_id == tenant_id)
            .cloned())
    }

    async fn save_ledger_hash_state(
        &self,
        state: LedgerHashState,
    ) -> Result<LedgerHashState, ApiError> {
        let mut inner = self.inner.lock();
        inner
            .ledger_hash_states
            .insert(state.period_id, state.clone());
        Ok(state)
    }

    async fn find_ledger_hash_state(
        &self,
        period_id: i64,
        tenant_id: i64,
    ) -> Result<Option<LedgerHashState>, ApiError> {
        let inner = self.inner.lock();
        Ok(inner
            .ledger_hash_states
            .get(&period_id)
            .filter(|s| s.tenant_id == tenant_id)
            .cloned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn sample_entry(period_id: i64, tenant_id: i64, entry_id: i64) -> HashChainEntry {
        HashChainEntry {
            id: 0,
            tenant_id,
            period_id,
            entry_id,
            previous_hash: None,
            entry_hash: format!("hash-{}", entry_id),
            created_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn test_create_and_find_hash_entries() {
        let repo = InMemoryBlockchainLedgerRepository::new();
        let entry = repo
            .create_hash_entry(sample_entry(1, 1, 101))
            .await
            .unwrap();
        assert_eq!(entry.id, 1);

        let entries = repo.find_hash_entries(1, 1).await.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].entry_id, 101);
    }

    #[tokio::test]
    async fn test_find_last_hash_entry() {
        let repo = InMemoryBlockchainLedgerRepository::new();
        repo.create_hash_entry(sample_entry(1, 1, 101))
            .await
            .unwrap();
        repo.create_hash_entry(sample_entry(1, 1, 102))
            .await
            .unwrap();

        let last = repo.find_last_hash_entry(1, 1).await.unwrap();
        assert_eq!(last.unwrap().entry_id, 102);
    }

    #[tokio::test]
    async fn test_tenant_isolation_hash_entries() {
        let repo = InMemoryBlockchainLedgerRepository::new();
        repo.create_hash_entry(sample_entry(1, 1, 101))
            .await
            .unwrap();
        repo.create_hash_entry(sample_entry(1, 2, 201))
            .await
            .unwrap();

        let tenant1 = repo.find_hash_entries(1, 1).await.unwrap();
        let tenant2 = repo.find_hash_entries(1, 2).await.unwrap();
        assert_eq!(tenant1.len(), 1);
        assert_eq!(tenant2.len(), 1);
        assert_eq!(tenant1[0].entry_id, 101);
        assert_eq!(tenant2[0].entry_id, 201);
    }

    #[tokio::test]
    async fn test_save_and_find_merkle_tree() {
        let repo = InMemoryBlockchainLedgerRepository::new();
        let tree = MerkleTree {
            period_id: 1,
            tenant_id: 1,
            root_hash: "root123".to_string(),
            leaf_count: 4,
            created_at: Utc::now(),
        };

        repo.save_merkle_tree(tree.clone()).await.unwrap();
        let found = repo.find_merkle_tree(1, 1).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().root_hash, "root123");

        let other_tenant = repo.find_merkle_tree(1, 999).await.unwrap();
        assert!(other_tenant.is_none());
    }

    #[tokio::test]
    async fn test_save_and_find_ledger_hash_state() {
        let repo = InMemoryBlockchainLedgerRepository::new();
        let state = LedgerHashState {
            period_id: 1,
            tenant_id: 1,
            merkle_root: "root123".to_string(),
            entry_count: 10,
            first_entry_hash: Some("first".to_string()),
            last_entry_hash: Some("last".to_string()),
            generated_at: Utc::now(),
        };

        repo.save_ledger_hash_state(state.clone()).await.unwrap();
        let found = repo.find_ledger_hash_state(1, 1).await.unwrap();
        assert!(found.is_some());
        let found = found.unwrap();
        assert_eq!(found.merkle_root, "root123");
        assert_eq!(found.entry_count, 10);

        let other_tenant = repo.find_ledger_hash_state(1, 999).await.unwrap();
        assert!(other_tenant.is_none());
    }
}
