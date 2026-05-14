//! Blockchain ledger models for e-Defter hash-chain compliance
//!
//! Provides types for cryptographic hash chaining and Merkle tree construction
//! to ensure immutable audit trails for Turkish electronic ledger entries.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Hash chain entry linking Yevmiye entries cryptographically
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct HashChainEntry {
    pub id: i64,
    pub tenant_id: i64,
    pub period_id: i64,
    pub entry_id: i64,
    pub previous_hash: Option<String>,
    pub entry_hash: String,
    pub created_at: DateTime<Utc>,
}

/// Merkle tree node for binary tree construction
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MerkleNode {
    pub hash: String,
    pub left: Option<Box<MerkleNode>>,
    pub right: Option<Box<MerkleNode>>,
}

/// Merkle tree summary for a ledger period
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MerkleTree {
    pub period_id: i64,
    pub tenant_id: i64,
    pub root_hash: String,
    pub leaf_count: usize,
    pub created_at: DateTime<Utc>,
}

/// Ledger hash state summary for a period
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LedgerHashState {
    pub period_id: i64,
    pub tenant_id: i64,
    pub merkle_root: String,
    pub entry_count: usize,
    pub first_entry_hash: Option<String>,
    pub last_entry_hash: Option<String>,
    pub generated_at: DateTime<Utc>,
}

/// Hash verification result for integrity checking
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct VerifyResult {
    pub period_id: i64,
    pub is_valid: bool,
    pub merkle_root_matches: bool,
    pub chain_intact: bool,
    pub broken_at_entry: Option<i64>,
    pub errors: Vec<String>,
}

/// Request to build Merkle tree for a period
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct BuildMerkleTreeRequest {
    pub period_id: i64,
    pub tenant_id: i64,
}

/// Request to verify a period's integrity
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct VerifyPeriodRequest {
    pub period_id: i64,
    pub tenant_id: i64,
}

/// Response for hash chain listing
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct HashChainResponse {
    pub period_id: i64,
    pub entries: Vec<HashChainEntry>,
    pub count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verify_result_defaults() {
        let result = VerifyResult {
            period_id: 1,
            is_valid: true,
            merkle_root_matches: true,
            chain_intact: true,
            broken_at_entry: None,
            errors: vec![],
        };
        assert!(result.is_valid);
        assert!(result.merkle_root_matches);
        assert!(result.chain_intact);
        assert!(result.broken_at_entry.is_none());
    }

    #[test]
    fn test_merkle_tree_creation() {
        let tree = MerkleTree {
            period_id: 1,
            tenant_id: 1,
            root_hash: "abc123".to_string(),
            leaf_count: 4,
            created_at: Utc::now(),
        };
        assert_eq!(tree.leaf_count, 4);
        assert_eq!(tree.root_hash, "abc123");
    }

    #[test]
    fn test_ledger_hash_state() {
        let state = LedgerHashState {
            period_id: 1,
            tenant_id: 1,
            merkle_root: "root123".to_string(),
            entry_count: 10,
            first_entry_hash: Some("first".to_string()),
            last_entry_hash: Some("last".to_string()),
            generated_at: Utc::now(),
        };
        assert_eq!(state.entry_count, 10);
        assert_eq!(state.first_entry_hash, Some("first".to_string()));
    }
}
