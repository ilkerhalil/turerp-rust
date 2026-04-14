//! Common pagination types and utilities

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Pagination query parameters
#[derive(Debug, Deserialize, ToSchema, utoipa::IntoParams)]
pub struct PaginationParams {
    /// Page number (1-based, default: 1)
    #[serde(default = "default_page")]
    pub page: u32,
    /// Number of items per page (default: 20, max: 100)
    #[serde(default = "default_per_page")]
    pub per_page: u32,
}

pub fn default_page() -> u32 {
    1
}

pub fn default_per_page() -> u32 {
    20
}

impl Default for PaginationParams {
    fn default() -> Self {
        Self {
            page: default_page(),
            per_page: default_per_page(),
        }
    }
}

impl PaginationParams {
    /// Validate pagination parameters
    pub fn validate(&self) -> Result<(), String> {
        if self.page == 0 {
            return Err("page must be at least 1".to_string());
        }
        if self.per_page == 0 || self.per_page > 100 {
            return Err("per_page must be between 1 and 100".to_string());
        }
        Ok(())
    }

    /// Calculate the offset for database queries
    pub fn offset(&self) -> u32 {
        (self.page.saturating_sub(1)) * self.per_page
    }

    /// Calculate the limit for database queries
    pub fn limit(&self) -> u32 {
        self.per_page
    }
}

/// Paginated result wrapper
#[derive(Debug, Serialize, ToSchema)]
pub struct PaginatedResult<T> {
    /// The items on this page
    pub items: Vec<T>,
    /// Current page number (1-based)
    pub page: u32,
    /// Number of items per page
    pub per_page: u32,
    /// Total number of items
    pub total: u64,
    /// Total number of pages
    pub total_pages: u32,
}

impl<T> PaginatedResult<T> {
    /// Create a new paginated result
    pub fn new(items: Vec<T>, page: u32, per_page: u32, total: u64) -> Self {
        let total_pages = if per_page > 0 {
            ((total as f64) / (per_page as f64)).ceil() as u32
        } else {
            0
        };

        Self {
            items,
            page,
            per_page,
            total,
            total_pages,
        }
    }

    /// Check if there is a next page
    pub fn has_next_page(&self) -> bool {
        self.page < self.total_pages
    }

    /// Check if there is a previous page
    pub fn has_previous_page(&self) -> bool {
        self.page > 1
    }

    /// Map the items to a different type, preserving pagination metadata
    pub fn map<U, F>(self, f: F) -> PaginatedResult<U>
    where
        F: FnMut(T) -> U,
    {
        PaginatedResult {
            items: self.items.into_iter().map(f).collect(),
            page: self.page,
            per_page: self.per_page,
            total: self.total,
            total_pages: self.total_pages,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pagination_params_default() {
        let params = PaginationParams::default();
        assert_eq!(params.page, 1);
        assert_eq!(params.per_page, 20);
    }

    #[test]
    fn test_pagination_params_validate() {
        let valid = PaginationParams {
            page: 1,
            per_page: 20,
        };
        assert!(valid.validate().is_ok());

        let invalid_page = PaginationParams {
            page: 0,
            per_page: 20,
        };
        assert!(invalid_page.validate().is_err());

        let invalid_per_page = PaginationParams {
            page: 1,
            per_page: 101,
        };
        assert!(invalid_per_page.validate().is_err());
    }

    #[test]
    fn test_offset_calculation() {
        let params = PaginationParams {
            page: 1,
            per_page: 20,
        };
        assert_eq!(params.offset(), 0);

        let params = PaginationParams {
            page: 2,
            per_page: 20,
        };
        assert_eq!(params.offset(), 20);

        let params = PaginationParams {
            page: 3,
            per_page: 10,
        };
        assert_eq!(params.offset(), 20);
    }

    #[test]
    fn test_paginated_result() {
        let items = vec![1, 2, 3];
        let result = PaginatedResult::new(items, 1, 20, 100);

        assert_eq!(result.page, 1);
        assert_eq!(result.per_page, 20);
        assert_eq!(result.total, 100);
        assert_eq!(result.total_pages, 5);
        assert!(result.has_next_page());
        assert!(!result.has_previous_page());
    }

    #[test]
    fn test_paginated_result_last_page() {
        let items = vec![1, 2, 3];
        let result = PaginatedResult::new(items, 5, 20, 100);

        assert_eq!(result.total_pages, 5);
        assert!(!result.has_next_page());
        assert!(result.has_previous_page());
    }
}
