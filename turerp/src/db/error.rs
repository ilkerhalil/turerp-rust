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

    #[test]
    fn test_map_sqlx_error_row_not_found() {
        let err = sqlx::Error::RowNotFound;
        let result = map_sqlx_error(err, "User");
        assert!(matches!(result, ApiError::NotFound(msg) if msg == "User not found"));
    }

    #[test]
    fn test_map_sqlx_error_unique_violation() {
        // Create a DatabaseError with code "23505"
        let db_err = DatabaseError::new(
            Some(std::borrow::Cow::Borrowed("23505")),
            std::borrow::Cow::Borrowed("unique constraint violation"),
        );
        let err = sqlx::Error::Database(Box::new(db_err));
        let result = map_sqlx_error(err, "User");
        assert!(matches!(result, ApiError::Conflict(msg) if msg == "User already exists"));
    }

    #[test]
    fn test_map_sqlx_error_foreign_key_violation() {
        let db_err = DatabaseError::new(
            Some(std::borrow::Cow::Borrowed("23503")),
            std::borrow::Cow::Borrowed("foreign key constraint violation"),
        );
        let err = sqlx::Error::Database(Box::new(db_err));
        let result = map_sqlx_error(err, "Cari");
        assert!(matches!(result, ApiError::BadRequest(msg) if msg.contains("Referenced")));
    }

    #[test]
    fn test_map_sqlx_error_generic_database() {
        let db_err = DatabaseError::new(
            Some(std::borrow::Cow::Borrowed("08006")),
            std::borrow::Cow::Borrowed("connection failure"),
        );
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
