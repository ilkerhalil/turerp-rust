//! PostgreSQL blockchain ledger repository implementation

use async_trait::async_trait;
use sqlx::{FromRow, PgPool};
use std::sync::Arc;

use crate::db::error::map_sqlx_error;
use crate::domain::edefter::blockchain::model::{HashChainEntry, LedgerHashState, MerkleTree};
use crate::domain::edefter::blockchain::repository::{
    BlockchainLedgerRepository, BoxBlockchainLedgerRepository,
};
use crate::error::ApiError;

// --- HashChainEntry ---

#[derive(Debug, FromRow)]
struct HashChainEntryRow {
    id: i64,
    tenant_id: i64,
    period_id: i64,
    entry_id: i64,
    previous_hash: Option<String>,
    entry_hash: String,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl From<HashChainEntryRow> for HashChainEntry {
    fn from(row: HashChainEntryRow) -> Self {
        Self {
            id: row.id,
            tenant_id: row.tenant_id,
            period_id: row.period_id,
            entry_id: row.entry_id,
            previous_hash: row.previous_hash,
            entry_hash: row.entry_hash,
            created_at: row.created_at,
        }
    }
}

// --- MerkleTree ---

#[derive(Debug, FromRow)]
struct MerkleTreeRow {
    #[allow(dead_code)]
    id: i64,
    tenant_id: i64,
    period_id: i64,
    root_hash: String,
    leaf_count: i32,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl From<MerkleTreeRow> for MerkleTree {
    fn from(row: MerkleTreeRow) -> Self {
        Self {
            period_id: row.period_id,
            tenant_id: row.tenant_id,
            root_hash: row.root_hash,
            leaf_count: row.leaf_count as usize,
            created_at: row.created_at,
        }
    }
}

// --- LedgerHashState ---

#[derive(Debug, FromRow)]
struct LedgerHashStateRow {
    #[allow(dead_code)]
    id: i64,
    tenant_id: i64,
    period_id: i64,
    merkle_root: String,
    entry_count: i32,
    first_entry_hash: Option<String>,
    last_entry_hash: Option<String>,
    generated_at: chrono::DateTime<chrono::Utc>,
}

impl From<LedgerHashStateRow> for LedgerHashState {
    fn from(row: LedgerHashStateRow) -> Self {
        Self {
            period_id: row.period_id,
            tenant_id: row.tenant_id,
            merkle_root: row.merkle_root,
            entry_count: row.entry_count as usize,
            first_entry_hash: row.first_entry_hash,
            last_entry_hash: row.last_entry_hash,
            generated_at: row.generated_at,
        }
    }
}

/// PostgreSQL blockchain ledger repository
pub struct PostgresBlockchainLedgerRepository {
    pool: Arc<PgPool>,
}

impl PostgresBlockchainLedgerRepository {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    pub fn into_boxed(self) -> BoxBlockchainLedgerRepository {
        Arc::new(self) as BoxBlockchainLedgerRepository
    }
}

#[async_trait]
impl BlockchainLedgerRepository for PostgresBlockchainLedgerRepository {
    async fn create_hash_entry(&self, entry: HashChainEntry) -> Result<HashChainEntry, ApiError> {
        let row: HashChainEntryRow = sqlx::query_as(
            r#"
            INSERT INTO blockchain_hash_entries (
                tenant_id, period_id, entry_id, previous_hash, entry_hash, created_at
            )
            VALUES ($1, $2, $3, $4, $5, NOW())
            RETURNING id, tenant_id, period_id, entry_id, previous_hash, entry_hash, created_at
            "#,
        )
        .bind(entry.tenant_id)
        .bind(entry.period_id)
        .bind(entry.entry_id)
        .bind(&entry.previous_hash)
        .bind(&entry.entry_hash)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "HashChainEntry"))?;

        Ok(row.into())
    }

    async fn find_hash_entries(
        &self,
        period_id: i64,
        tenant_id: i64,
    ) -> Result<Vec<HashChainEntry>, ApiError> {
        let rows: Vec<HashChainEntryRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, period_id, entry_id, previous_hash, entry_hash, created_at
            FROM blockchain_hash_entries
            WHERE period_id = $1 AND tenant_id = $2
            ORDER BY id ASC
            "#,
        )
        .bind(period_id)
        .bind(tenant_id)
        .fetch_all(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find hash entries: {}", e)))?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn find_last_hash_entry(
        &self,
        period_id: i64,
        tenant_id: i64,
    ) -> Result<Option<HashChainEntry>, ApiError> {
        let result: Option<HashChainEntryRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, period_id, entry_id, previous_hash, entry_hash, created_at
            FROM blockchain_hash_entries
            WHERE period_id = $1 AND tenant_id = $2
            ORDER BY id DESC
            LIMIT 1
            "#,
        )
        .bind(period_id)
        .bind(tenant_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find last hash entry: {}", e)))?;

        Ok(result.map(|r| r.into()))
    }

    async fn save_merkle_tree(&self, tree: MerkleTree) -> Result<MerkleTree, ApiError> {
        let row: MerkleTreeRow = sqlx::query_as(
            r#"
            INSERT INTO blockchain_merkle_trees (
                tenant_id, period_id, root_hash, leaf_count, created_at
            )
            VALUES ($1, $2, $3, $4, NOW())
            ON CONFLICT (period_id) DO UPDATE SET
                tenant_id = EXCLUDED.tenant_id,
                root_hash = EXCLUDED.root_hash,
                leaf_count = EXCLUDED.leaf_count,
                created_at = EXCLUDED.created_at
            RETURNING id, tenant_id, period_id, root_hash, leaf_count, created_at
            "#,
        )
        .bind(tree.tenant_id)
        .bind(tree.period_id)
        .bind(&tree.root_hash)
        .bind(tree.leaf_count as i32)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "MerkleTree"))?;

        Ok(row.into())
    }

    async fn find_merkle_tree(
        &self,
        period_id: i64,
        tenant_id: i64,
    ) -> Result<Option<MerkleTree>, ApiError> {
        let result: Option<MerkleTreeRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, period_id, root_hash, leaf_count, created_at
            FROM blockchain_merkle_trees
            WHERE period_id = $1 AND tenant_id = $2
            "#,
        )
        .bind(period_id)
        .bind(tenant_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find Merkle tree: {}", e)))?;

        Ok(result.map(|r| r.into()))
    }

    async fn save_ledger_hash_state(
        &self,
        state: LedgerHashState,
    ) -> Result<LedgerHashState, ApiError> {
        let row: LedgerHashStateRow = sqlx::query_as(
            r#"
            INSERT INTO blockchain_ledger_hash_states (
                tenant_id, period_id, merkle_root, entry_count, first_entry_hash, last_entry_hash, generated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, NOW())
            ON CONFLICT (period_id) DO UPDATE SET
                tenant_id = EXCLUDED.tenant_id,
                merkle_root = EXCLUDED.merkle_root,
                entry_count = EXCLUDED.entry_count,
                first_entry_hash = EXCLUDED.first_entry_hash,
                last_entry_hash = EXCLUDED.last_entry_hash,
                generated_at = EXCLUDED.generated_at
            RETURNING id, tenant_id, period_id, merkle_root, entry_count, first_entry_hash, last_entry_hash, generated_at
            "#,
        )
        .bind(state.tenant_id)
        .bind(state.period_id)
        .bind(&state.merkle_root)
        .bind(state.entry_count as i32)
        .bind(&state.first_entry_hash)
        .bind(&state.last_entry_hash)
        .fetch_one(&*self.pool)
        .await
        .map_err(|e| map_sqlx_error(e, "LedgerHashState"))?;

        Ok(row.into())
    }

    async fn find_ledger_hash_state(
        &self,
        period_id: i64,
        tenant_id: i64,
    ) -> Result<Option<LedgerHashState>, ApiError> {
        let result: Option<LedgerHashStateRow> = sqlx::query_as(
            r#"
            SELECT id, tenant_id, period_id, merkle_root, entry_count, first_entry_hash, last_entry_hash, generated_at
            FROM blockchain_ledger_hash_states
            WHERE period_id = $1 AND tenant_id = $2
            "#,
        )
        .bind(period_id)
        .bind(tenant_id)
        .fetch_optional(&*self.pool)
        .await
        .map_err(|e| ApiError::Database(format!("Failed to find ledger hash state: {}", e)))?;

        Ok(result.map(|r| r.into()))
    }
}
