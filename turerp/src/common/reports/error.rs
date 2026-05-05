//! Report error types

use thiserror::Error;

/// Errors that can occur during report generation
#[derive(Error, Debug)]
pub enum ReportError {
    #[error("Report {0} not found")]
    NotFound(i64),
    #[error("Invalid format: {0}")]
    InvalidFormat(String),
    #[error("Generation failed: {0}")]
    GenerationFailed(String),
    #[error("Template not found: {0}")]
    TemplateNotFound(String),
    #[error("IO error: {0}")]
    Io(String),
}

impl From<rust_xlsxwriter::XlsxError> for ReportError {
    fn from(err: rust_xlsxwriter::XlsxError) -> Self {
        ReportError::GenerationFailed(format!("Excel error: {}", err))
    }
}

impl From<ReportError> for crate::error::ApiError {
    fn from(err: ReportError) -> Self {
        match err {
            ReportError::NotFound(id) => Self::NotFound(format!("Report {id} not found")),
            ReportError::InvalidFormat(msg) => Self::Validation(msg),
            ReportError::GenerationFailed(msg) => Self::Internal(msg),
            ReportError::TemplateNotFound(msg) => Self::NotFound(msg),
            ReportError::Io(msg) => Self::Internal(msg),
        }
    }
}
