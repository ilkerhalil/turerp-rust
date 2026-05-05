//! Database error handling utilities

use crate::error::ApiError;

/// PostgreSQL error code for unique constraint violation
const PG_UNIQUE_VIOLATION: &str = "23505";

/// PostgreSQL error code for foreign key constraint violation
const PG_FOREIGN_KEY_VIOLATION: &str = "23503";

/// Convert sqlx errors to ApiError with proper detection of PostgreSQL error codes.
///
/// Uses PostgreSQL error codes instead of string matching for reliable detection:
/// - `23505` = unique constraint violation → Conflict
/// - `23503` = foreign key violation → BadRequest
/// - `RowNotFound` → NotFound
/// - Everything else → Database (internal, logged, sanitized for client)
pub fn map_sqlx_error(e: sqlx::Error, entity: &str) -> ApiError {
    match &e {
        sqlx::Error::RowNotFound => ApiError::NotFound(format!("{} not found", entity)),
        sqlx::Error::Database(db_err) => match db_err.code().as_deref() {
            Some(PG_UNIQUE_VIOLATION) => ApiError::Conflict(format!("{} already exists", entity)),
            Some(PG_FOREIGN_KEY_VIOLATION) => {
                ApiError::BadRequest(format!("Referenced {} record does not exist", entity))
            }
            _ => ApiError::Database(format!("Failed to operate on {}: {}", entity, e)),
        },
        _ => ApiError::Database(format!("Failed to operate on {}: {}", entity, e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::error::DatabaseError;
    use std::borrow::Cow;

    #[derive(Debug)]
    struct MockDbError {
        code: Option<String>,
        message: String,
    }

    impl std::fmt::Display for MockDbError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.message)
        }
    }

    impl std::error::Error for MockDbError {}

    impl DatabaseError for MockDbError {
        fn message(&self) -> &str {
            &self.message
        }

        fn code(&self) -> Option<Cow<'_, str>> {
            self.code.as_deref().map(Cow::Borrowed)
        }

        fn as_error(&self) -> &(dyn std::error::Error + Send + Sync + 'static) {
            self
        }

        fn as_error_mut(&mut self) -> &mut (dyn std::error::Error + Send + Sync + 'static) {
            self
        }

        fn into_error(self: Box<Self>) -> Box<dyn std::error::Error + Send + Sync + 'static> {
            self
        }

        fn kind(&self) -> sqlx::error::ErrorKind {
            sqlx::error::ErrorKind::Other
        }
    }

    #[test]
    fn test_map_sqlx_error_row_not_found() {
        let err = sqlx::Error::RowNotFound;
        let result = map_sqlx_error(err, "User");
        assert!(matches!(result, ApiError::NotFound(msg) if msg == "User not found"));
    }

    #[test]
    fn test_map_sqlx_error_unique_violation() {
        let db_err = MockDbError {
            code: Some("23505".to_string()),
            message: "unique constraint violation".to_string(),
        };
        let err = sqlx::Error::Database(Box::new(db_err));
        let result = map_sqlx_error(err, "User");
        assert!(matches!(result, ApiError::Conflict(msg) if msg == "User already exists"));
    }

    #[test]
    fn test_map_sqlx_error_foreign_key_violation() {
        let db_err = MockDbError {
            code: Some("23503".to_string()),
            message: "foreign key constraint violation".to_string(),
        };
        let err = sqlx::Error::Database(Box::new(db_err));
        let result = map_sqlx_error(err, "Cari");
        assert!(matches!(result, ApiError::BadRequest(msg) if msg.contains("Referenced")));
    }

    #[test]
    fn test_map_sqlx_error_generic_database() {
        let db_err = MockDbError {
            code: Some("08006".to_string()),
            message: "connection failure".to_string(),
        };
        let err = sqlx::Error::Database(Box::new(db_err));
        let result = map_sqlx_error(err, "Invoice");
        assert!(matches!(result, ApiError::Database(_)));
    }

    #[test]
    fn test_map_sqlx_error_pool_timeout() {
        let err = sqlx::Error::PoolTimedOut;
        let result = map_sqlx_error(err, "Account");
        assert!(matches!(result, ApiError::Database(_)));
    }
}
