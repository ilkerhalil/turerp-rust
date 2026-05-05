//! Report generation engine for PDF, Excel, and XML exports
//!
//! Provides a `ReportEngine` trait with in-memory and pluggable backends.
//! Supports invoice PDF, accounting/HR Excel, and e-Defter UBL-TR XML.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub mod csv;
pub mod engine;
pub mod error;
pub mod excel;
pub mod pdf;
pub mod template;
pub mod xml;

pub use engine::InMemoryReportEngine;
pub use error::ReportError;
pub use template::ReportTemplate;

/// Report format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReportFormat {
    Pdf,
    Excel,
    Xml,
    Csv,
    Json,
}

/// Report type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReportType {
    Invoice,
    TrialBalance,
    BalanceSheet,
    IncomeStatement,
    PayrollSummary,
    StockSummary,
    SalesReport,
    PurchaseReport,
    AgingReport,
    EDefter,
    Custom(String),
}

/// Report generation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportRequest {
    pub report_type: ReportType,
    pub format: ReportFormat,
    pub tenant_id: i64,
    pub title: String,
    pub parameters: serde_json::Value,
    pub requested_by: Option<i64>,
    pub locale: Option<String>,
}

/// Generated report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedReport {
    pub id: i64,
    pub report_type: ReportType,
    pub format: ReportFormat,
    pub tenant_id: i64,
    pub title: String,
    pub data: Vec<u8>,
    pub filename: String,
    pub content_type: String,
    pub generated_at: DateTime<Utc>,
    pub generated_by: Option<i64>,
}

/// Report metadata (without binary data)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportMeta {
    pub id: i64,
    pub report_type: ReportType,
    pub format: ReportFormat,
    pub tenant_id: i64,
    pub title: String,
    pub filename: String,
    pub size_bytes: i64,
    pub generated_at: DateTime<Utc>,
    pub generated_by: Option<i64>,
}

impl From<&GeneratedReport> for ReportMeta {
    fn from(r: &GeneratedReport) -> Self {
        Self {
            id: r.id,
            report_type: r.report_type.clone(),
            format: r.format,
            tenant_id: r.tenant_id,
            title: r.title.clone(),
            filename: r.filename.clone(),
            size_bytes: r.data.len() as i64,
            generated_at: r.generated_at,
            generated_by: r.generated_by,
        }
    }
}

/// Content type by format
impl ReportFormat {
    pub fn content_type(&self) -> &'static str {
        match self {
            Self::Pdf => "application/pdf",
            Self::Excel => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
            Self::Xml => "application/xml",
            Self::Csv => "text/csv",
            Self::Json => "application/json",
        }
    }

    pub fn extension(&self) -> &'static str {
        match self {
            Self::Pdf => "pdf",
            Self::Excel => "xlsx",
            Self::Xml => "xml",
            Self::Csv => "csv",
            Self::Json => "json",
        }
    }
}

/// Display for report type
impl std::fmt::Display for ReportType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Invoice => write!(f, "invoice"),
            Self::TrialBalance => write!(f, "trial_balance"),
            Self::BalanceSheet => write!(f, "balance_sheet"),
            Self::IncomeStatement => write!(f, "income_statement"),
            Self::PayrollSummary => write!(f, "payroll_summary"),
            Self::StockSummary => write!(f, "stock_summary"),
            Self::SalesReport => write!(f, "sales_report"),
            Self::PurchaseReport => write!(f, "purchase_report"),
            Self::AgingReport => write!(f, "aging_report"),
            Self::EDefter => write!(f, "edefter"),
            Self::Custom(name) => write!(f, "custom_{}", name),
        }
    }
}

/// Report engine trait
#[async_trait::async_trait]
pub trait ReportEngine: Send + Sync {
    /// Generate a report
    async fn generate(&self, request: ReportRequest) -> Result<GeneratedReport, ReportError>;

    /// Get a previously generated report
    async fn get_report(
        &self,
        tenant_id: i64,
        report_id: i64,
    ) -> Result<Option<GeneratedReport>, ReportError>;

    /// List report metadata for a tenant
    async fn list_reports(
        &self,
        tenant_id: i64,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<ReportMeta>, ReportError>;

    /// Delete a report
    async fn delete_report(&self, tenant_id: i64, report_id: i64) -> Result<(), ReportError>;

    /// Store mapping between a background job and generated report
    async fn store_job_mapping(&self, _job_id: i64, _report_id: i64) -> Result<(), ReportError> {
        Ok(())
    }

    /// Get report ID for a background job
    async fn get_report_for_job(&self, _job_id: i64) -> Result<Option<i64>, ReportError> {
        Ok(None)
    }
}

/// Type alias for boxed report engine
pub type BoxReportEngine = std::sync::Arc<dyn ReportEngine>;
