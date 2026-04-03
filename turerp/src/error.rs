//! Error types for the Turerp ERP system

use actix_web::{HttpResponse, ResponseError};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// API Error types
#[derive(Error, Debug)]
pub enum ApiError {
    #[error("Database error: {0}")]
    Database(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Internal server error: {0}")]
    Internal(String),

    #[error("Invalid credentials")]
    InvalidCredentials,

    #[error("Token expired")]
    TokenExpired,

    #[error("Invalid token: {0}")]
    InvalidToken(String),

    #[error("Validation error: {0}")]
    Validation(String),
}

/// Error response structure for JSON API responses
#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
}

impl From<std::num::ParseIntError> for ApiError {
    fn from(err: std::num::ParseIntError) -> Self {
        ApiError::BadRequest(err.to_string())
    }
}

impl ResponseError for ApiError {
    fn error_response(&self) -> HttpResponse {
        match self {
            ApiError::NotFound(msg) => {
                HttpResponse::NotFound().json(ErrorResponse { error: msg.clone() })
            }
            ApiError::Unauthorized(msg) => {
                HttpResponse::Unauthorized().json(ErrorResponse { error: msg.clone() })
            }
            ApiError::Forbidden(msg) => {
                HttpResponse::Forbidden().json(ErrorResponse { error: msg.clone() })
            }
            ApiError::BadRequest(msg) => {
                HttpResponse::BadRequest().json(ErrorResponse { error: msg.clone() })
            }
            ApiError::Conflict(msg) => {
                HttpResponse::Conflict().json(ErrorResponse { error: msg.clone() })
            }
            ApiError::InvalidCredentials => HttpResponse::Unauthorized().json(ErrorResponse {
                error: "Invalid username or password".to_string(),
            }),
            ApiError::TokenExpired => HttpResponse::Unauthorized().json(ErrorResponse {
                error: "Token has expired".to_string(),
            }),
            ApiError::InvalidToken(msg) => {
                HttpResponse::Unauthorized().json(ErrorResponse { error: msg.clone() })
            }
            ApiError::Validation(msg) => {
                HttpResponse::BadRequest().json(ErrorResponse { error: msg.clone() })
            }
            ApiError::Database(msg) => {
                tracing::error!("Database error: {}", msg);
                HttpResponse::InternalServerError().json(ErrorResponse {
                    error: "An internal database error occurred".to_string(),
                })
            }
            ApiError::Internal(msg) => {
                tracing::error!("Internal error: {}", msg);
                HttpResponse::InternalServerError().json(ErrorResponse {
                    error: "An internal error occurred".to_string(),
                })
            }
        }
    }
}

/// Result type alias for API operations
pub type ApiResult<T> = Result<T, ApiError>;

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::http::StatusCode;

    #[test]
    fn test_api_error_display() {
        let err = ApiError::NotFound("User not found".to_string());
        assert_eq!(err.to_string(), "Not found: User not found");

        let err = ApiError::Unauthorized("Invalid token".to_string());
        assert_eq!(err.to_string(), "Unauthorized: Invalid token");

        let err = ApiError::Forbidden("Admin access required".to_string());
        assert_eq!(err.to_string(), "Forbidden: Admin access required");

        let err = ApiError::BadRequest("Invalid input".to_string());
        assert_eq!(err.to_string(), "Bad request: Invalid input");

        let err = ApiError::Conflict("Email exists".to_string());
        assert_eq!(err.to_string(), "Conflict: Email exists");

        let err = ApiError::Internal("DB error".to_string());
        assert_eq!(err.to_string(), "Internal server error: DB error");
    }

    #[test]
    fn test_error_response_status_codes() {
        // NotFound -> 404
        let response = ApiError::NotFound("Resource not found".to_string()).error_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        // Unauthorized -> 401
        let response = ApiError::Unauthorized("Token expired".to_string()).error_response();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        // Forbidden -> 403
        let response = ApiError::Forbidden("Admin access required".to_string()).error_response();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);

        // InvalidCredentials -> 401
        let response = ApiError::InvalidCredentials.error_response();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        // TokenExpired -> 401
        let response = ApiError::TokenExpired.error_response();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        // InvalidToken -> 401
        let response = ApiError::InvalidToken("Malformed".to_string()).error_response();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        // BadRequest -> 400
        let response = ApiError::BadRequest("Invalid input".to_string()).error_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        // Validation -> 400
        let response = ApiError::Validation("Invalid field".to_string()).error_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        // Conflict -> 409
        let response = ApiError::Conflict("Duplicate".to_string()).error_response();
        assert_eq!(response.status(), StatusCode::CONFLICT);

        // Database -> 500
        let response = ApiError::Database("Connection failed".to_string()).error_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

        // Internal -> 500
        let response = ApiError::Internal("Unexpected error".to_string()).error_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn test_parse_int_error_conversion() {
        let result: Result<i32, _> = "abc".parse();
        let err: ApiError = result.unwrap_err().into();

        assert!(matches!(err, ApiError::BadRequest(_)));
    }

    #[test]
    fn test_error_response_serialization() {
        let response = ErrorResponse {
            error: "Test error".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("Test error"));

        let deserialized: ErrorResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.error, "Test error");
    }

    #[test]
    fn test_error_response_deserialization() {
        let json = r#"{"error":"Something went wrong"}"#;
        let response: ErrorResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.error, "Something went wrong");
    }

    #[test]
    fn test_api_result_ok() {
        let result: ApiResult<i32> = Ok(42);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_api_result_err() {
        let result: ApiResult<i32> = Err(ApiError::NotFound("Not found".to_string()));
        assert!(result.is_err());
    }

    #[test]
    fn test_api_error_variants() {
        // Test that all error variants can be created and converted to string
        assert!(!ApiError::Database("err".to_string()).to_string().is_empty());
        assert!(!ApiError::NotFound("err".to_string()).to_string().is_empty());
        assert!(!ApiError::Unauthorized("err".to_string())
            .to_string()
            .is_empty());
        assert!(!ApiError::Forbidden("err".to_string())
            .to_string()
            .is_empty());
        assert!(!ApiError::BadRequest("err".to_string())
            .to_string()
            .is_empty());
        assert!(!ApiError::Conflict("err".to_string()).to_string().is_empty());
        assert!(!ApiError::Internal("err".to_string()).to_string().is_empty());
        assert!(!ApiError::InvalidCredentials.to_string().is_empty());
        assert!(!ApiError::TokenExpired.to_string().is_empty());
        assert!(!ApiError::InvalidToken("err".to_string())
            .to_string()
            .is_empty());
        assert!(!ApiError::Validation("err".to_string())
            .to_string()
            .is_empty());
    }
}
