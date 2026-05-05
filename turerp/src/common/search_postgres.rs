//! PostgreSQL-backed full-text search service using pg_trgm, unaccent, and Turkish tsvector.

use async_trait::async_trait;
use sqlx::{FromRow, PgPool};
use std::sync::Arc;

use crate::common::search::{SearchDocument, SearchQuery, SearchResult, SearchService};
use crate::error::ApiError;

/// Row returned by the unified cross-entity search query.
#[derive(Debug, FromRow)]
struct SearchRow {
    entity_type: String,
    id: i64,
    tenant_id: i64,
    title: String,
    description: Option<String>,
    score: f64,
}

/// PostgreSQL search service backed by native GIN indexes.
///
/// Uses `pg_trgm` for fuzzy trigram matching, `unaccent` for accent-insensitive
/// search, and `to_tsvector('turkish', ...)` for Turkish full-text ranking.
/// `index` / `remove` / `reindex` are no-ops because the database maintains
/// the `search_vector` and GIN indexes automatically via triggers.
pub struct PostgresSearchService {
    pool: Arc<PgPool>,
}

impl PostgresSearchService {
    /// Create a new PostgreSQL search service.
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    /// Build the accent-insensitive, Turkish-aware ranked search query.
    fn build_search_sql(entity_types: &[String]) -> String {
        let mut parts = Vec::new();

        let cari_sql = r#"
            SELECT 'cari'::text AS entity_type,
                   id,
                   tenant_id,
                   name AS title,
                   NULL::text AS description,
                   GREATEST(
                       similarity(unaccent(name), unaccent($1)),
                       similarity(unaccent(code), unaccent($1)),
                       COALESCE(ts_rank_cd(search_vector, plainto_tsquery('turkish', $1), 32), 0.0)
                   )::float8 AS score
            FROM cari
            WHERE tenant_id = $2
              AND deleted_at IS NULL
              AND (
                  unaccent(name) % unaccent($1)
                  OR unaccent(code) % unaccent($1)
                  OR search_vector @@ plainto_tsquery('turkish', $1)
              )
        "#;

        let product_sql = r#"
            SELECT 'product'::text AS entity_type,
                   id,
                   tenant_id,
                   name AS title,
                   description,
                   GREATEST(
                       similarity(unaccent(name), unaccent($1)),
                       similarity(unaccent(code), unaccent($1)),
                       COALESCE(ts_rank_cd(search_vector, plainto_tsquery('turkish', $1), 32), 0.0)
                   )::float8 AS score
            FROM products
            WHERE tenant_id = $2
              AND deleted_at IS NULL
              AND (
                  unaccent(name) % unaccent($1)
                  OR unaccent(code) % unaccent($1)
                  OR search_vector @@ plainto_tsquery('turkish', $1)
              )
        "#;

        let invoice_sql = r#"
            SELECT 'invoice'::text AS entity_type,
                   id,
                   tenant_id,
                   invoice_number AS title,
                   notes AS description,
                   GREATEST(
                       similarity(unaccent(invoice_number), unaccent($1)),
                       similarity(unaccent(COALESCE(notes, '')), unaccent($1)),
                       COALESCE(ts_rank_cd(search_vector, plainto_tsquery('turkish', $1), 32), 0.0)
                   )::float8 AS score
            FROM invoices
            WHERE tenant_id = $2
              AND deleted_at IS NULL
              AND (
                  unaccent(invoice_number) % unaccent($1)
                  OR unaccent(COALESCE(notes, '')) % unaccent($1)
                  OR search_vector @@ plainto_tsquery('turkish', $1)
              )
        "#;

        if entity_types.is_empty() {
            parts.push(cari_sql.to_string());
            parts.push(product_sql.to_string());
            parts.push(invoice_sql.to_string());
        } else {
            for et in entity_types {
                match et.to_lowercase().as_str() {
                    "cari" => parts.push(cari_sql.to_string()),
                    "product" => parts.push(product_sql.to_string()),
                    "invoice" => parts.push(invoice_sql.to_string()),
                    _ => {}
                }
            }
        }

        if parts.is_empty() {
            // If no valid entity types, return zero rows with correct schema.
            return r#"
                SELECT NULL::text AS entity_type,
                       0::bigint AS id,
                       0::bigint AS tenant_id,
                       ''::text AS title,
                       NULL::text AS description,
                       0.0::float8 AS score
                WHERE FALSE
            "#
            .to_string();
        }

        format!("{} ORDER BY score DESC LIMIT $3", parts.join(" UNION ALL "))
    }
}

#[async_trait]
impl SearchService for PostgresSearchService {
    async fn search(&self, query: SearchQuery) -> Result<Vec<SearchResult>, ApiError> {
        query
            .validate()
            .map_err(|e| ApiError::Validation(e.join(", ")))?;

        let limit = query.limit.unwrap_or(20) as i64;
        let sql = Self::build_search_sql(&query.entity_types);

        let rows: Vec<SearchRow> = sqlx::query_as(&sql)
            .bind(&query.query)
            .bind(query.tenant_id)
            .bind(limit)
            .fetch_all(&*self.pool)
            .await
            .map_err(|e| ApiError::Database(format!("Full-text search failed: {}", e)))?;

        let min_score = query.min_score.unwrap_or(0.1);
        let results: Vec<SearchResult> = rows
            .into_iter()
            .filter(|r| r.score >= min_score)
            .map(|r| SearchResult {
                entity_type: r.entity_type,
                id: r.id,
                tenant_id: r.tenant_id,
                title: r.title,
                description: r.description,
                score: r.score,
            })
            .collect();

        Ok(results)
    }

    /// No-op: GIN indexes and triggers maintain the index automatically.
    async fn index(&self, _doc: &SearchDocument) -> Result<(), ApiError> {
        Ok(())
    }

    /// No-op: GIN indexes and triggers maintain the index automatically.
    async fn remove(
        &self,
        _entity_type: &str,
        _entity_id: i64,
        _tenant_id: i64,
    ) -> Result<(), ApiError> {
        Ok(())
    }

    /// No-op: GIN indexes and triggers maintain the index automatically.
    async fn reindex(&self, _tenant_id: i64) -> Result<(), ApiError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_search_sql_empty_entity_types() {
        let sql = PostgresSearchService::build_search_sql(&[]);
        assert!(sql.contains("cari"));
        assert!(sql.contains("product"));
        assert!(sql.contains("invoice"));
        assert!(sql.contains("ORDER BY score DESC LIMIT $3"));
    }

    #[test]
    fn test_build_search_sql_filtered() {
        let sql = PostgresSearchService::build_search_sql(&["cari".to_string()]);
        assert!(sql.contains("cari"));
        assert!(!sql.contains("product"));
        assert!(!sql.contains("invoice"));
    }

    #[test]
    fn test_build_search_sql_multiple_filtered() {
        let sql =
            PostgresSearchService::build_search_sql(&["cari".to_string(), "invoice".to_string()]);
        assert!(sql.contains("cari"));
        assert!(!sql.contains("product"));
        assert!(sql.contains("invoice"));
    }

    #[test]
    fn test_build_search_sql_invalid_ignored() {
        let sql = PostgresSearchService::build_search_sql(&["unknown".to_string()]);
        assert!(!sql.contains("cari"));
        assert!(!sql.contains("product"));
        assert!(!sql.contains("invoice"));
        assert!(sql.contains("WHERE FALSE"));
    }
}
