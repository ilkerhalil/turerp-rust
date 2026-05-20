//! Blockchain ledger service for e-Defter hash-chain compliance
//!
//! Provides SHA-256 hash generation, Merkle tree construction,
//! and integrity verification for Turkish electronic ledger entries.

use chrono::Utc;
use sha2::{Digest, Sha256};

use crate::domain::edefter::blockchain::model::*;
use crate::domain::edefter::blockchain::repository::BoxBlockchainLedgerRepository;
use crate::domain::edefter::model::YevmiyeEntry;
use crate::error::ApiError;

/// Service for managing blockchain ledger hash chains and Merkle trees
#[derive(Clone)]
pub struct BlockchainLedgerService {
    repo: BoxBlockchainLedgerRepository,
}

impl BlockchainLedgerService {
    pub fn new(repo: BoxBlockchainLedgerRepository) -> Self {
        Self { repo }
    }

    /// Compute SHA-256 hash for a Yevmiye entry.
    ///
    /// The hash is computed from a canonical representation:
    /// `{entry_number}:{entry_date}:{explanation}:{debit}:{credit}:{lines_hash}:{previous_hash}`
    ///
    /// Where `lines_hash` is the SHA-256 of lines serialized as:
    /// `account_code:account_name:debit:credit:explanation|...`
    pub fn compute_entry_hash(entry: &YevmiyeEntry, previous_hash: Option<&str>) -> String {
        let lines_str = entry
            .lines
            .iter()
            .map(|line| {
                format!(
                    "{}:{}:{}:{}:{}",
                    line.account_code, line.account_name, line.debit, line.credit, line.explanation
                )
            })
            .collect::<Vec<_>>()
            .join("|");

        let lines_hash = Self::sha256_hex(lines_str.as_bytes());

        let canonical = format!(
            "{}:{}:{}:{}:{}:{}:{}",
            entry.entry_number,
            entry.entry_date,
            entry.explanation,
            entry.debit_total,
            entry.credit_total,
            lines_hash,
            previous_hash.unwrap_or("GENESIS")
        );

        Self::sha256_hex(canonical.as_bytes())
    }

    /// Build a hash chain for a set of Yevmiye entries.
    ///
    /// Each entry gets a `HashChainEntry` with its hash and the previous
    /// entry's hash, creating an immutable linked sequence.
    pub async fn build_hash_chain(
        &self,
        tenant_id: i64,
        period_id: i64,
        entries: Vec<YevmiyeEntry>,
    ) -> Result<Vec<HashChainEntry>, ApiError> {
        let mut chain = Vec::with_capacity(entries.len());
        let mut previous_hash: Option<String> = None;

        for entry in entries {
            let entry_hash = Self::compute_entry_hash(&entry, previous_hash.as_deref());

            let chain_entry = HashChainEntry {
                id: 0,
                tenant_id,
                period_id,
                entry_id: entry.id,
                previous_hash: previous_hash.clone(),
                entry_hash: entry_hash.clone(),
                created_at: Utc::now(),
            };

            let stored = self.repo.create_hash_entry(chain_entry).await?;
            chain.push(stored);
            previous_hash = Some(entry_hash);
        }

        Ok(chain)
    }

    /// Build a Merkle tree from a list of entry hashes.
    ///
    /// Uses standard binary Merkle tree construction:
    /// 1. Leaves are the entry hashes
    /// 2. Odd-length layers duplicate the last hash
    /// 3. Each parent = SHA-256(left + right)
    pub async fn build_merkle_tree(
        &self,
        tenant_id: i64,
        period_id: i64,
        entry_hashes: Vec<String>,
    ) -> Result<MerkleTree, ApiError> {
        if entry_hashes.is_empty() {
            return Err(ApiError::BadRequest(
                "Cannot build Merkle tree from empty hash list".to_string(),
            ));
        }

        let root_hash = Self::compute_merkle_root(&entry_hashes);

        let tree = MerkleTree {
            period_id,
            tenant_id,
            root_hash,
            leaf_count: entry_hashes.len(),
            created_at: Utc::now(),
        };

        self.repo.save_merkle_tree(tree.clone()).await?;

        let state = LedgerHashState {
            period_id,
            tenant_id,
            merkle_root: tree.root_hash.clone(),
            entry_count: entry_hashes.len(),
            first_entry_hash: entry_hashes.first().cloned(),
            last_entry_hash: entry_hashes.last().cloned(),
            generated_at: Utc::now(),
        };
        self.repo.save_ledger_hash_state(state).await?;

        Ok(tree)
    }

    /// Verify the integrity of a period's entries against stored hashes.
    ///
    /// Checks:
    /// 1. Hash chain continuity (each entry's previous_hash matches)
    /// 2. Entry hash correctness (re-computed hash matches stored)
    /// 3. Merkle root match (if expected root provided)
    pub async fn verify_period_integrity(
        &self,
        tenant_id: i64,
        period_id: i64,
        entries: Vec<YevmiyeEntry>,
        expected_merkle_root: Option<String>,
    ) -> Result<VerifyResult, ApiError> {
        let stored_chain = self.repo.find_hash_entries(period_id, tenant_id).await?;
        let mut errors = Vec::new();
        let mut chain_intact = true;
        let mut merkle_root_matches = true;
        let mut broken_at_entry: Option<i64> = None;

        // Build a map of entry_id -> stored hash chain entry for O(1) lookup
        let stored_map: std::collections::HashMap<i64, &HashChainEntry> =
            stored_chain.iter().map(|e| (e.entry_id, e)).collect();

        let mut previous_hash: Option<String> = None;

        for entry in &entries {
            match stored_map.get(&entry.id) {
                Some(stored) => {
                    // Verify previous hash linkage
                    if stored.previous_hash != previous_hash {
                        chain_intact = false;
                        broken_at_entry = Some(entry.id);
                        errors.push(format!(
                            "Entry {}: previous_hash mismatch (expected {:?}, found {:?})",
                            entry.id, previous_hash, stored.previous_hash
                        ));
                    }

                    // Verify entry hash correctness
                    let computed = Self::compute_entry_hash(entry, previous_hash.as_deref());
                    if computed != stored.entry_hash {
                        chain_intact = false;
                        if broken_at_entry.is_none() {
                            broken_at_entry = Some(entry.id);
                        }
                        errors.push(format!(
                            "Entry {}: hash mismatch (expected {}, found {})",
                            entry.id, computed, stored.entry_hash
                        ));
                    }

                    previous_hash = Some(stored.entry_hash.clone());
                }
                None => {
                    chain_intact = false;
                    if broken_at_entry.is_none() {
                        broken_at_entry = Some(entry.id);
                    }
                    errors.push(format!("Entry {}: no hash chain record found", entry.id));
                }
            }
        }

        // Verify Merkle root if expected value provided
        if let Some(expected) = expected_merkle_root {
            let entry_hashes: Vec<String> = entries
                .iter()
                .map(|e| {
                    stored_map
                        .get(&e.id)
                        .map(|s| s.entry_hash.clone())
                        .unwrap_or_else(|| Self::compute_entry_hash(e, None))
                })
                .collect();
            let computed_root = Self::compute_merkle_root(&entry_hashes);
            if computed_root != expected {
                merkle_root_matches = false;
                errors.push(format!(
                    "Merkle root mismatch (expected {}, found {})",
                    expected, computed_root
                ));
            }
        }

        let is_valid = chain_intact && merkle_root_matches && errors.is_empty();

        Ok(VerifyResult {
            period_id,
            is_valid,
            merkle_root_matches,
            chain_intact,
            broken_at_entry,
            errors,
        })
    }

    /// Get the stored ledger hash state for a period.
    pub async fn get_ledger_hash_state(
        &self,
        tenant_id: i64,
        period_id: i64,
    ) -> Result<Option<LedgerHashState>, ApiError> {
        self.repo.find_ledger_hash_state(period_id, tenant_id).await
    }

    /// Get the hash chain for a period.
    pub async fn get_hash_chain(
        &self,
        tenant_id: i64,
        period_id: i64,
    ) -> Result<Vec<HashChainEntry>, ApiError> {
        self.repo.find_hash_entries(period_id, tenant_id).await
    }

    /// Get the stored Merkle tree for a period.
    pub async fn get_merkle_tree(
        &self,
        tenant_id: i64,
        period_id: i64,
    ) -> Result<Option<MerkleTree>, ApiError> {
        self.repo.find_merkle_tree(period_id, tenant_id).await
    }

    // -----------------------------------------------------------------------
    // Private helpers
    // -----------------------------------------------------------------------

    fn sha256_hex(data: &[u8]) -> String {
        let digest = Sha256::digest(data);
        hex::encode(digest)
    }

    fn compute_merkle_root(hashes: &[String]) -> String {
        if hashes.is_empty() {
            return Self::sha256_hex(b"");
        }
        if hashes.len() == 1 {
            return hashes[0].clone();
        }

        let mut current = hashes.to_vec();

        while current.len() > 1 {
            let mut next = Vec::new();

            for chunk in current.chunks(2) {
                let left = &chunk[0];
                let right = if chunk.len() == 2 {
                    &chunk[1]
                } else {
                    left // duplicate last hash for odd count
                };

                let combined = format!("{}{}", left, right);
                let parent = Self::sha256_hex(combined.as_bytes());
                next.push(parent);
            }

            current = next;
        }

        current
            .into_iter()
            .next()
            .unwrap_or_else(|| Self::sha256_hex(b""))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::edefter::blockchain::repository::InMemoryBlockchainLedgerRepository;
    use crate::domain::edefter::model::YevmiyeLine;
    use chrono::NaiveDate;
    use rust_decimal::Decimal;
    use std::sync::Arc;

    fn make_service() -> BlockchainLedgerService {
        let repo =
            Arc::new(InMemoryBlockchainLedgerRepository::new()) as BoxBlockchainLedgerRepository;
        BlockchainLedgerService::new(repo)
    }

    fn sample_entry(id: i64, entry_number: i64, explanation: &str) -> YevmiyeEntry {
        YevmiyeEntry {
            id,
            period_id: 1,
            entry_number,
            entry_date: NaiveDate::from_ymd_opt(2024, 6, 15).unwrap(),
            explanation: explanation.to_string(),
            debit_total: Decimal::new(10000, 2),
            credit_total: Decimal::new(10000, 2),
            lines: vec![YevmiyeLine {
                account_code: "100.01".to_string(),
                account_name: "Kasa".to_string(),
                debit: Decimal::new(10000, 2),
                credit: Decimal::ZERO,
                explanation: "Test line".to_string(),
            }],
        }
    }

    #[test]
    fn test_compute_entry_hash_deterministic() {
        let entry = sample_entry(1, 1, "Test");
        let hash1 = BlockchainLedgerService::compute_entry_hash(&entry, None);
        let hash2 = BlockchainLedgerService::compute_entry_hash(&entry, None);
        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 64); // SHA-256 hex = 64 chars
    }

    #[test]
    fn test_compute_entry_hash_with_previous() {
        let entry = sample_entry(1, 1, "Test");
        let prev = "abcd1234";
        let hash_with = BlockchainLedgerService::compute_entry_hash(&entry, Some(prev));
        let hash_without = BlockchainLedgerService::compute_entry_hash(&entry, None);
        assert_ne!(hash_with, hash_without);
    }

    #[test]
    fn test_compute_entry_hash_different_entries() {
        let entry1 = sample_entry(1, 1, "First");
        let entry2 = sample_entry(2, 2, "Second");
        let hash1 = BlockchainLedgerService::compute_entry_hash(&entry1, None);
        let hash2 = BlockchainLedgerService::compute_entry_hash(&entry2, None);
        assert_ne!(hash1, hash2);
    }

    #[tokio::test]
    async fn test_build_hash_chain() {
        let svc = make_service();
        let entries = vec![
            sample_entry(1, 1, "First"),
            sample_entry(2, 2, "Second"),
            sample_entry(3, 3, "Third"),
        ];

        let chain = svc.build_hash_chain(1, 1, entries).await.unwrap();
        assert_eq!(chain.len(), 3);
        assert!(chain[0].previous_hash.is_none());
        assert_eq!(chain[1].previous_hash, Some(chain[0].entry_hash.clone()));
        assert_eq!(chain[2].previous_hash, Some(chain[1].entry_hash.clone()));
    }

    #[tokio::test]
    async fn test_build_merkle_tree() {
        let svc = make_service();
        let hashes = vec![
            "hash-a".to_string(),
            "hash-b".to_string(),
            "hash-c".to_string(),
            "hash-d".to_string(),
        ];

        let tree = svc.build_merkle_tree(1, 1, hashes).await.unwrap();
        assert_eq!(tree.leaf_count, 4);
        assert_eq!(tree.root_hash.len(), 64);
        assert_eq!(tree.period_id, 1);

        let state = svc.get_ledger_hash_state(1, 1).await.unwrap();
        assert!(state.is_some());
        assert_eq!(state.unwrap().merkle_root, tree.root_hash);
    }

    #[tokio::test]
    async fn test_build_merkle_tree_empty_fails() {
        let svc = make_service();
        let result = svc.build_merkle_tree(1, 1, vec![]).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_verify_period_integrity_valid() {
        let svc = make_service();
        let entries = vec![sample_entry(1, 1, "First"), sample_entry(2, 2, "Second")];

        let chain = svc.build_hash_chain(1, 1, entries.clone()).await.unwrap();
        let hashes: Vec<String> = chain.iter().map(|c| c.entry_hash.clone()).collect();
        let tree = svc.build_merkle_tree(1, 1, hashes.clone()).await.unwrap();

        let result = svc
            .verify_period_integrity(1, 1, entries, Some(tree.root_hash.clone()))
            .await
            .unwrap();
        assert!(result.is_valid);
        assert!(result.chain_intact);
        assert!(result.merkle_root_matches);
        assert!(result.errors.is_empty());
    }

    #[tokio::test]
    async fn test_verify_period_integrity_tampered() {
        let svc = make_service();
        let entries = vec![sample_entry(1, 1, "First"), sample_entry(2, 2, "Second")];

        svc.build_hash_chain(1, 1, entries.clone()).await.unwrap();

        // Tamper with the second entry
        let mut tampered = entries;
        tampered[1].explanation = "Tampered!".to_string();

        let result = svc
            .verify_period_integrity(1, 1, tampered, None)
            .await
            .unwrap();
        assert!(!result.is_valid);
        assert!(!result.chain_intact);
        assert_eq!(result.broken_at_entry, Some(2));
        assert!(!result.errors.is_empty());
    }

    #[tokio::test]
    async fn test_verify_merkle_root_mismatch() {
        let svc = make_service();
        let entries = vec![sample_entry(1, 1, "First")];

        svc.build_hash_chain(1, 1, entries.clone()).await.unwrap();

        let result = svc
            .verify_period_integrity(1, 1, entries, Some("wrong-root".to_string()))
            .await
            .unwrap();
        assert!(!result.is_valid);
        assert!(!result.merkle_root_matches);
    }

    #[tokio::test]
    async fn test_get_hash_chain() {
        let svc = make_service();
        let entries = vec![sample_entry(1, 1, "First"), sample_entry(2, 2, "Second")];

        svc.build_hash_chain(1, 1, entries).await.unwrap();
        let chain = svc.get_hash_chain(1, 1).await.unwrap();
        assert_eq!(chain.len(), 2);
    }

    #[tokio::test]
    async fn test_get_merkle_tree() {
        let svc = make_service();
        let hashes = vec!["a".to_string(), "b".to_string()];
        svc.build_merkle_tree(1, 1, hashes).await.unwrap();

        let tree = svc.get_merkle_tree(1, 1).await.unwrap();
        assert!(tree.is_some());
    }
}
