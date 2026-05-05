//! Full-text search service with PostgreSQL pg_trgm and in-memory fallback
//!
//! Provides a `SearchService` trait for fuzzy text search across entities.
//! Uses PostgreSQL `pg_trgm` and `unaccent` extensions for production,
//! and a simple in-memory trigram-based search for development.

use crate::error::ApiError;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Search result item
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SearchResult {
    /// Entity type (e.g., "cari", "product", "invoice")
    pub entity_type: String,
    /// Entity ID
    pub id: i64,
    /// Tenant ID
    pub tenant_id: i64,
    /// Primary display text (name, code, etc.)
    pub title: String,
    /// Secondary display text (description, notes, etc.)
    pub description: Option<String>,
    /// Relevance score (0.0 - 1.0, higher is better)
    pub score: f64,
}

/// Search query parameters
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SearchQuery {
    /// Search text
    pub query: String,
    /// Tenant ID for isolation
    pub tenant_id: i64,
    /// Entity types to search (empty = all)
    pub entity_types: Vec<String>,
    /// Maximum results per entity type
    pub limit: Option<u32>,
    /// Minimum relevance score (0.0 - 1.0)
    pub min_score: Option<f64>,
}

impl SearchQuery {
    /// Validate search query
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        if self.query.trim().is_empty() {
            errors.push("Search query cannot be empty".to_string());
        }
        if self.query.len() > 200 {
            errors.push("Search query must be at most 200 characters".to_string());
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Create a new search query for a tenant
    pub fn new(query: impl Into<String>, tenant_id: i64) -> Self {
        Self {
            query: query.into(),
            tenant_id,
            entity_types: Vec::new(),
            limit: Some(20),
            min_score: Some(0.1),
        }
    }

    /// Filter to specific entity types
    pub fn with_entity_types(mut self, types: Vec<String>) -> Self {
        self.entity_types = types;
        self
    }

    /// Set result limit
    pub fn with_limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }
}

/// Search service trait
#[async_trait::async_trait]
pub trait SearchService: Send + Sync {
    /// Search across all indexed entities
    async fn search(&self, query: SearchQuery) -> Result<Vec<SearchResult>, ApiError>;

    /// Index a document for search
    async fn index(&self, doc: &SearchDocument) -> Result<(), ApiError>;

    /// Remove a document from the search index
    async fn remove(
        &self,
        entity_type: &str,
        entity_id: i64,
        tenant_id: i64,
    ) -> Result<(), ApiError>;

    /// Reindex all data for a tenant
    async fn reindex(&self, tenant_id: i64) -> Result<(), ApiError>;
}

/// Document to be indexed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchDocument {
    pub entity_type: String,
    pub id: i64,
    pub tenant_id: i64,
    pub title: String,
    pub description: Option<String>,
    pub searchable_text: String,
}

/// In-memory search service using simple fuzzy matching
pub struct InMemorySearchService {
    documents: parking_lot::RwLock<Vec<SearchDocument>>,
}

impl InMemorySearchService {
    pub fn new() -> Self {
        Self {
            documents: parking_lot::RwLock::new(Vec::new()),
        }
    }

    /// Simple trigram-based similarity score
    fn trigram_similarity(a: &str, b: &str) -> f64 {
        let a_lower = a.to_lowercase();
        let b_lower = b.to_lowercase();

        if a_lower.is_empty() || b_lower.is_empty() {
            return 0.0;
        }

        if a_lower == b_lower {
            return 1.0;
        }

        if a_lower.contains(&b_lower) || b_lower.contains(&a_lower) {
            return 0.8;
        }

        // Check word-level overlap
        let a_words: std::collections::HashSet<&str> = a_lower.split_whitespace().collect();
        let b_words: std::collections::HashSet<&str> = b_lower.split_whitespace().collect();
        let intersection = a_words.intersection(&b_words).count();
        let union = a_words.union(&b_words).count();

        if union == 0 {
            return 0.0;
        }

        let jaccard = intersection as f64 / union as f64;

        // Also check for prefix matches
        let a_prefix: String = a_lower.chars().take(3).collect();
        let b_prefix: String = b_lower.chars().take(3).collect();
        let prefix_match = if a_prefix == b_prefix { 0.2 } else { 0.0 };

        (jaccard + prefix_match).min(1.0)
    }

    fn matches_entity_type(query: &SearchQuery, entity_type: &str) -> bool {
        if query.entity_types.is_empty() {
            return true;
        }
        query
            .entity_types
            .iter()
            .any(|t| t.eq_ignore_ascii_case(entity_type))
    }
}

impl Default for InMemorySearchService {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl SearchService for InMemorySearchService {
    async fn search(&self, query: SearchQuery) -> Result<Vec<SearchResult>, ApiError> {
        query
            .validate()
            .map_err(|e| ApiError::Validation(e.join(", ")))?;

        let limit = query.limit.unwrap_or(20) as usize;
        let min_score = query.min_score.unwrap_or(0.1);
        let documents = self.documents.read();

        let mut results: Vec<SearchResult> = documents
            .iter()
            .filter(|doc| {
                doc.tenant_id == query.tenant_id
                    && Self::matches_entity_type(&query, &doc.entity_type)
            })
            .map(|doc| {
                let title_score = Self::trigram_similarity(&query.query, &doc.title);
                let desc_score = doc
                    .description
                    .as_ref()
                    .map(|d| Self::trigram_similarity(&query.query, d))
                    .unwrap_or(0.0);
                let text_score = Self::trigram_similarity(&query.query, &doc.searchable_text);
                let score = title_score.max(desc_score).max(text_score * 0.7);

                SearchResult {
                    entity_type: doc.entity_type.clone(),
                    id: doc.id,
                    tenant_id: doc.tenant_id,
                    title: doc.title.clone(),
                    description: doc.description.clone(),
                    score,
                }
            })
            .filter(|r| r.score >= min_score)
            .collect();

        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        results.truncate(limit);
        Ok(results)
    }

    async fn index(&self, doc: &SearchDocument) -> Result<(), ApiError> {
        let mut documents = self.documents.write();
        // Remove existing document with same entity_type/id/tenant_id
        documents.retain(|d| {
            !(d.entity_type == doc.entity_type && d.id == doc.id && d.tenant_id == doc.tenant_id)
        });
        documents.push(doc.clone());
        Ok(())
    }

    async fn remove(
        &self,
        entity_type: &str,
        entity_id: i64,
        tenant_id: i64,
    ) -> Result<(), ApiError> {
        let mut documents = self.documents.write();
        documents.retain(|d| {
            !(d.entity_type == entity_type && d.id == entity_id && d.tenant_id == tenant_id)
        });
        Ok(())
    }

    async fn reindex(&self, tenant_id: i64) -> Result<(), ApiError> {
        // In-memory reindex is a no-op; data would need to be re-populated
        // from the domain repositories
        let mut documents = self.documents.write();
        documents.retain(|d| d.tenant_id != tenant_id);
        Ok(())
    }
}

/// Type alias for boxed search service
pub type BoxSearchService = std::sync::Arc<dyn SearchService>;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_search_basic() {
        let service = InMemorySearchService::new();

        service
            .index(&SearchDocument {
                entity_type: "cari".to_string(),
                id: 1,
                tenant_id: 1,
                title: "Acme Corporation".to_string(),
                description: Some("A large technology company".to_string()),
                searchable_text: "Acme Corporation technology".to_string(),
            })
            .await
            .unwrap();

        service
            .index(&SearchDocument {
                entity_type: "cari".to_string(),
                id: 2,
                tenant_id: 1,
                title: "Beta Industries".to_string(),
                description: Some("Manufacturing company".to_string()),
                searchable_text: "Beta Industries manufacturing".to_string(),
            })
            .await
            .unwrap();

        let query = SearchQuery::new("Acme", 1);
        let results = service.search(query).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, 1);
    }

    #[tokio::test]
    async fn test_search_tenant_isolation() {
        let service = InMemorySearchService::new();

        service
            .index(&SearchDocument {
                entity_type: "cari".to_string(),
                id: 1,
                tenant_id: 1,
                title: "Acme Corp".to_string(),
                description: None,
                searchable_text: "Acme Corp".to_string(),
            })
            .await
            .unwrap();

        service
            .index(&SearchDocument {
                entity_type: "cari".to_string(),
                id: 2,
                tenant_id: 2,
                title: "Acme Different".to_string(),
                description: None,
                searchable_text: "Acme Different".to_string(),
            })
            .await
            .unwrap();

        let results = service.search(SearchQuery::new("Acme", 1)).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].tenant_id, 1);
    }

    #[tokio::test]
    async fn test_search_entity_type_filter() {
        let service = InMemorySearchService::new();

        service
            .index(&SearchDocument {
                entity_type: "cari".to_string(),
                id: 1,
                tenant_id: 1,
                title: "Acme Corp".to_string(),
                description: None,
                searchable_text: "Acme Corp cari".to_string(),
            })
            .await
            .unwrap();

        service
            .index(&SearchDocument {
                entity_type: "product".to_string(),
                id: 1,
                tenant_id: 1,
                title: "Acme Product".to_string(),
                description: None,
                searchable_text: "Acme Product item".to_string(),
            })
            .await
            .unwrap();

        let results = service
            .search(SearchQuery::new("Acme", 1).with_entity_types(vec!["cari".to_string()]))
            .await
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].entity_type, "cari");
    }

    #[tokio::test]
    async fn test_search_remove() {
        let service = InMemorySearchService::new();

        service
            .index(&SearchDocument {
                entity_type: "cari".to_string(),
                id: 1,
                tenant_id: 1,
                title: "Acme Corp".to_string(),
                description: None,
                searchable_text: "Acme Corp".to_string(),
            })
            .await
            .unwrap();

        service.remove("cari", 1, 1).await.unwrap();

        let results = service.search(SearchQuery::new("Acme", 1)).await.unwrap();
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn test_search_fuzzy() {
        let service = InMemorySearchService::new();

        service
            .index(&SearchDocument {
                entity_type: "cari".to_string(),
                id: 1,
                tenant_id: 1,
                title: "İstanbul Büyükşehir".to_string(),
                description: None,
                searchable_text: "istanbul buyuksehir belediyesi".to_string(),
            })
            .await
            .unwrap();

        // Partial match
        let results = service.search(SearchQuery::new("İstanb", 1)).await.unwrap();
        assert!(!results.is_empty());
    }

    #[tokio::test]
    async fn test_search_validation() {
        let query = SearchQuery::new("", 1);
        assert!(query.validate().is_err());

        let query = SearchQuery {
            query: "a".repeat(201),
            tenant_id: 1,
            entity_types: vec![],
            limit: Some(20),
            min_score: Some(0.1),
        };
        assert!(query.validate().is_err());

        let query = SearchQuery::new("test", 1);
        assert!(query.validate().is_ok());
    }
}
