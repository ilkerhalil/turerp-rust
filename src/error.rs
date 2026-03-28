//! Error types for the Turerp ERP system

use actix_web::{HttpResponse, ResponseError};
use serde::Serialize;
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
#[derive(Debug, Serialize)]
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
