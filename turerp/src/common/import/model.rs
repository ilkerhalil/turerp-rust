//! Import domain models and types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Supported entity types for bulk import/export
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum EntityType {
    Product,
    Cari,
    ChartOfAccounts,
    StockMovement,
}

impl std::fmt::Display for EntityType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EntityType::Product => write!(f, "product"),
            EntityType::Cari => write!(f, "cari"),
            EntityType::ChartOfAccounts => write!(f, "chart_of_accounts"),
            EntityType::StockMovement => write!(f, "stock_movement"),
        }
    }
}

impl std::str::FromStr for EntityType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "product" => Ok(EntityType::Product),
            "cari" => Ok(EntityType::Cari),
            "chart_of_accounts" => Ok(EntityType::ChartOfAccounts),
            "stock_movement" => Ok(EntityType::StockMovement),
            _ => Err(format!("Invalid entity type: {}", s)),
        }
    }
}

/// File format for import/export
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum ImportFormat {
    Csv,
    Json,
}

impl std::fmt::Display for ImportFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImportFormat::Csv => write!(f, "csv"),
            ImportFormat::Json => write!(f, "json"),
        }
    }
}

impl std::str::FromStr for ImportFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "csv" => Ok(ImportFormat::Csv),
            "json" => Ok(ImportFormat::Json),
            _ => Err(format!("Invalid import format: {}", s)),
        }
    }
}

/// Status of an import job
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub enum ImportStatus {
    Pending,
    Validating,
    Processing,
    Completed,
    Failed,
}

/// Single validation error for an import row
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ImportError {
    pub row: usize,
    pub field: Option<String>,
    pub message: String,
}

/// Result of an import operation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ImportResult {
    pub job_id: i64,
    pub entity_type: EntityType,
    pub status: ImportStatus,
    pub total_rows: usize,
    pub success_rows: usize,
    pub failed_rows: usize,
    pub errors: Vec<ImportError>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

impl ImportResult {
    pub fn new(job_id: i64, entity_type: EntityType) -> Self {
        Self {
            job_id,
            entity_type,
            status: ImportStatus::Pending,
            total_rows: 0,
            success_rows: 0,
            failed_rows: 0,
            errors: Vec::new(),
            created_at: Utc::now(),
            completed_at: None,
        }
    }

    pub fn with_error(
        mut self,
        row: usize,
        field: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        self.errors.push(ImportError {
            row,
            field: Some(field.into()),
            message: message.into(),
        });
        self.failed_rows += 1;
        self
    }

    pub fn add_success(&mut self) {
        self.success_rows += 1;
    }

    pub fn complete(mut self) -> Self {
        self.status = ImportStatus::Completed;
        self.completed_at = Some(Utc::now());
        self
    }

    pub fn fail(mut self, message: impl Into<String>) -> Self {
        self.status = ImportStatus::Failed;
        self.errors.push(ImportError {
            row: 0,
            field: None,
            message: message.into(),
        });
        self.completed_at = Some(Utc::now());
        self
    }
}

/// Row-level raw data for a product import
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProductImportRow {
    pub code: String,
    pub name: String,
    pub unit_price: String,
    pub category: Option<String>,
    pub unit: Option<String>,
}

/// Row-level raw data for a cari import
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CariImportRow {
    pub code: String,
    pub name: String,
    pub cari_type: String,
    pub tax_number: Option<String>,
    pub email: Option<String>,
}

/// Row-level raw data for chart of accounts import
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ChartAccountImportRow {
    pub code: String,
    pub name: String,
    pub account_type: String,
    pub parent_code: Option<String>,
}

/// Row-level raw data for stock movement import
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StockMovementImportRow {
    pub product_code: String,
    pub warehouse_id: String,
    pub quantity: String,
    pub direction: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_entity_type_from_str() {
        assert_eq!(
            EntityType::from_str("product").unwrap(),
            EntityType::Product
        );
        assert_eq!(EntityType::from_str("CARI").unwrap(), EntityType::Cari);
        assert!(EntityType::from_str("unknown").is_err());
    }

    #[test]
    fn test_import_format_from_str() {
        assert_eq!(ImportFormat::from_str("csv").unwrap(), ImportFormat::Csv);
        assert_eq!(ImportFormat::from_str("JSON").unwrap(), ImportFormat::Json);
        assert!(ImportFormat::from_str("xlsx").is_err());
    }

    #[test]
    fn test_import_result_tracking() {
        let result = ImportResult::new(1, EntityType::Product);
        assert_eq!(result.total_rows, 0);
        assert_eq!(result.success_rows, 0);
        assert_eq!(result.failed_rows, 0);

        let result = result.with_error(1, "code", "duplicate");
        assert_eq!(result.failed_rows, 1);
        assert_eq!(result.errors[0].row, 1);
    }
}
