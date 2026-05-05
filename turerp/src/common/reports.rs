//! Report generation engine for PDF, Excel, and XML exports
//!
//! Provides a `ReportEngine` trait with in-memory and pluggable backends.
//! Supports invoice PDF, accounting/HR Excel, and e-Defter UBL-TR XML.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

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
    async fn generate(&self, request: ReportRequest) -> Result<GeneratedReport, String>;

    /// Get a previously generated report
    async fn get_report(
        &self,
        tenant_id: i64,
        report_id: i64,
    ) -> Result<Option<GeneratedReport>, String>;

    /// List report metadata for a tenant
    async fn list_reports(
        &self,
        tenant_id: i64,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<ReportMeta>, String>;

    /// Delete a report
    async fn delete_report(&self, tenant_id: i64, report_id: i64) -> Result<(), String>;
}

/// Type alias for boxed report engine
pub type BoxReportEngine = std::sync::Arc<dyn ReportEngine>;

/// In-memory report engine (generates placeholder content)
pub struct InMemoryReportEngine {
    reports: parking_lot::RwLock<Vec<GeneratedReport>>,
    next_id: parking_lot::RwLock<i64>,
}

impl InMemoryReportEngine {
    pub fn new() -> Self {
        Self {
            reports: parking_lot::RwLock::new(Vec::new()),
            next_id: parking_lot::RwLock::new(1),
        }
    }

    fn allocate_id(&self) -> i64 {
        let mut id = self.next_id.write();
        let report_id = *id;
        *id += 1;
        report_id
    }

    fn generate_placeholder_data(request: &ReportRequest) -> Vec<u8> {
        match request.format {
            ReportFormat::Pdf => Self::generate_invoice_pdf(request),
            ReportFormat::Excel => Self::generate_excel(request),
            ReportFormat::Xml => Self::generate_edefter_xml(request),
            ReportFormat::Csv => Self::generate_csv(request),
            ReportFormat::Json => Self::generate_json(request),
        }
    }

    fn generate_invoice_pdf(request: &ReportRequest) -> Vec<u8> {
        let params = &request.parameters;
        let invoice_no = params
            .get("invoice_no")
            .and_then(|v| v.as_str())
            .unwrap_or("N/A");
        let total = params.get("total").and_then(|v| v.as_f64()).unwrap_or(0.0);

        format!(
            "%PDF-1.4\nInvoice Report\n\tenant: {}\n\tinvoice: {}\n\ttotal: {:.2}\n\tdate: {}\n%%EOF",
            request.tenant_id, invoice_no, total, Utc::now().to_rfc3339()
        ).into_bytes()
    }

    fn generate_excel(request: &ReportRequest) -> Vec<u8> {
        let params = &request.parameters;
        let rows = params
            .get("rows")
            .and_then(|v| v.as_array())
            .map(|a| a.len())
            .unwrap_or(0);

        format!(
            "PK..EXCEL\nReport: {}\nTenant: {}\nRows: {}\nDate: {}",
            request.title,
            request.tenant_id,
            rows,
            Utc::now().to_rfc3339()
        )
        .into_bytes()
    }

    fn generate_edefter_xml(request: &ReportRequest) -> Vec<u8> {
        let params = &request.parameters;
        let period = params
            .get("period")
            .and_then(|v| v.as_str())
            .unwrap_or("2026-01");

        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<GenericAccountingPacket xmlns="urn:gi:eFatura:ubl:GenericAccountingPacket">
  <PacketInfo>
    <PacketVersId>1</PacketVersId>
    <PacketType>GENELMUHASEBE</PacketType>
    <Period>{period}</Period>
    <TenantId>{tenant}</TenantId>
  </PacketInfo>
</GenericAccountingPacket>"#,
            period = period,
            tenant = request.tenant_id
        )
        .into_bytes()
    }

    fn generate_csv(request: &ReportRequest) -> Vec<u8> {
        let params = &request.parameters;
        let columns = params
            .get("columns")
            .and_then(|v| v.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str())
                    .collect::<Vec<_>>()
                    .join(",")
            })
            .unwrap_or_default();

        format!("{}\n{}", columns, request.title).into_bytes()
    }

    fn generate_json(request: &ReportRequest) -> Vec<u8> {
        serde_json::to_string_pretty(&request.parameters)
            .unwrap_or_else(|_| "{}".to_string())
            .into_bytes()
    }
}

impl Default for InMemoryReportEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl ReportEngine for InMemoryReportEngine {
    async fn generate(&self, request: ReportRequest) -> Result<GeneratedReport, String> {
        let id = self.allocate_id();
        let data = Self::generate_placeholder_data(&request);
        let filename = format!(
            "{}_{}_{}.{}",
            request.report_type,
            request.tenant_id,
            id,
            request.format.extension()
        );

        let report = GeneratedReport {
            id,
            report_type: request.report_type.clone(),
            format: request.format,
            tenant_id: request.tenant_id,
            title: request.title,
            data,
            filename,
            content_type: request.format.content_type().to_string(),
            generated_at: Utc::now(),
            generated_by: request.requested_by,
        };

        self.reports.write().push(report.clone());
        Ok(report)
    }

    async fn get_report(
        &self,
        tenant_id: i64,
        report_id: i64,
    ) -> Result<Option<GeneratedReport>, String> {
        Ok(self
            .reports
            .read()
            .iter()
            .find(|r| r.id == report_id && r.tenant_id == tenant_id)
            .cloned())
    }

    async fn list_reports(
        &self,
        tenant_id: i64,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<ReportMeta>, String> {
        let reports = self.reports.read();
        Ok(reports
            .iter()
            .filter(|r| r.tenant_id == tenant_id)
            .skip(offset as usize)
            .take(limit as usize)
            .map(ReportMeta::from)
            .collect())
    }

    async fn delete_report(&self, tenant_id: i64, report_id: i64) -> Result<(), String> {
        let mut reports = self.reports.write();
        let idx = reports
            .iter()
            .position(|r| r.id == report_id && r.tenant_id == tenant_id)
            .ok_or_else(|| format!("Report {} not found", report_id))?;
        reports.remove(idx);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_generate_invoice_pdf() {
        let engine = InMemoryReportEngine::new();
        let request = ReportRequest {
            report_type: ReportType::Invoice,
            format: ReportFormat::Pdf,
            tenant_id: 1,
            title: "Invoice #1".to_string(),
            parameters: serde_json::json!({
                "invoice_no": "INV-001",
                "total": 1500.50
            }),
            requested_by: Some(1),
            locale: Some("tr".to_string()),
        };

        let report = engine.generate(request).await.unwrap();
        assert_eq!(report.tenant_id, 1);
        assert_eq!(report.format, ReportFormat::Pdf);
        assert!(report.filename.ends_with(".pdf"));
        assert_eq!(report.content_type, "application/pdf");
        assert!(!report.data.is_empty());
    }

    #[tokio::test]
    async fn test_generate_edefter_xml() {
        let engine = InMemoryReportEngine::new();
        let request = ReportRequest {
            report_type: ReportType::EDefter,
            format: ReportFormat::Xml,
            tenant_id: 1,
            title: "e-Defter 2026-01".to_string(),
            parameters: serde_json::json!({ "period": "2026-01" }),
            requested_by: Some(1),
            locale: Some("tr".to_string()),
        };

        let report = engine.generate(request).await.unwrap();
        assert_eq!(report.format, ReportFormat::Xml);
        assert!(report.filename.ends_with(".xml"));
        let xml = String::from_utf8(report.data).unwrap();
        assert!(xml.contains("GenericAccountingPacket"));
    }

    #[tokio::test]
    async fn test_generate_excel() {
        let engine = InMemoryReportEngine::new();
        let request = ReportRequest {
            report_type: ReportType::TrialBalance,
            format: ReportFormat::Excel,
            tenant_id: 1,
            title: "Trial Balance".to_string(),
            parameters: serde_json::json!({ "rows": [{}] }),
            requested_by: None,
            locale: None,
        };

        let report = engine.generate(request).await.unwrap();
        assert_eq!(report.format, ReportFormat::Excel);
        assert!(report.filename.ends_with(".xlsx"));
    }

    #[tokio::test]
    async fn test_get_and_list_reports() {
        let engine = InMemoryReportEngine::new();

        for i in 0..3 {
            engine
                .generate(ReportRequest {
                    report_type: ReportType::Invoice,
                    format: ReportFormat::Pdf,
                    tenant_id: 1,
                    title: format!("Report {}", i),
                    parameters: serde_json::json!({}),
                    requested_by: None,
                    locale: None,
                })
                .await
                .unwrap();
        }

        let meta = engine.list_reports(1, 10, 0).await.unwrap();
        assert_eq!(meta.len(), 3);

        let report = engine.get_report(1, 1).await.unwrap();
        assert!(report.is_some());

        let cross_tenant = engine.get_report(2, 1).await.unwrap();
        assert!(cross_tenant.is_none());
    }

    #[tokio::test]
    async fn test_delete_report() {
        let engine = InMemoryReportEngine::new();
        engine
            .generate(ReportRequest {
                report_type: ReportType::Invoice,
                format: ReportFormat::Pdf,
                tenant_id: 1,
                title: "Delete me".to_string(),
                parameters: serde_json::json!({}),
                requested_by: None,
                locale: None,
            })
            .await
            .unwrap();

        engine.delete_report(1, 1).await.unwrap();
        let result = engine.get_report(1, 1).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_pdf_content_structure() {
        let engine = InMemoryReportEngine::new();
        let request = ReportRequest {
            report_type: ReportType::Invoice,
            format: ReportFormat::Pdf,
            tenant_id: 42,
            title: "Invoice #1".to_string(),
            parameters: serde_json::json!({
                "invoice_no": "INV-TEST-001",
                "total": 9999.99
            }),
            requested_by: Some(1),
            locale: Some("tr".to_string()),
        };

        let report = engine.generate(request).await.unwrap();
        let pdf_text = String::from_utf8_lossy(&report.data);
        assert!(pdf_text.contains("%PDF-1.4"));
        assert!(pdf_text.contains("Invoice Report"));
        assert!(pdf_text.contains("INV-TEST-001"));
        assert!(pdf_text.contains("9999.99"));
        assert!(pdf_text.contains("42")); // tenant_id appears in PDF
        assert!(pdf_text.contains("%%EOF"));
    }

    #[tokio::test]
    async fn test_excel_content_structure() {
        let engine = InMemoryReportEngine::new();
        let request = ReportRequest {
            report_type: ReportType::PayrollSummary,
            format: ReportFormat::Excel,
            tenant_id: 7,
            title: "Payroll Q1".to_string(),
            parameters: serde_json::json!({
                "rows": [{"name": "Alice"}, {"name": "Bob"}, {"name": "Charlie"}]
            }),
            requested_by: None,
            locale: None,
        };

        let report = engine.generate(request).await.unwrap();
        let excel_text = String::from_utf8_lossy(&report.data);
        assert!(excel_text.contains("PK..EXCEL"));
        assert!(excel_text.contains("Payroll Q1"));
        assert!(excel_text.contains("Tenant: 7"));
        assert!(excel_text.contains("Rows: 3"));
    }

    #[tokio::test]
    async fn test_csv_format_correctness() {
        let engine = InMemoryReportEngine::new();
        let request = ReportRequest {
            report_type: ReportType::SalesReport,
            format: ReportFormat::Csv,
            tenant_id: 1,
            title: "Monthly Sales".to_string(),
            parameters: serde_json::json!({
                "columns": ["Date", "Customer", "Amount", "Currency"]
            }),
            requested_by: None,
            locale: None,
        };

        let report = engine.generate(request).await.unwrap();
        let csv_text = String::from_utf8_lossy(&report.data);
        let lines: Vec<&str> = csv_text.lines().collect();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], "Date,Customer,Amount,Currency");
        assert_eq!(lines[1], "Monthly Sales");
    }

    #[tokio::test]
    async fn test_csv_empty_columns() {
        let engine = InMemoryReportEngine::new();
        let request = ReportRequest {
            report_type: ReportType::PurchaseReport,
            format: ReportFormat::Csv,
            tenant_id: 1,
            title: "Empty CSV".to_string(),
            parameters: serde_json::json!({}),
            requested_by: None,
            locale: None,
        };

        let report = engine.generate(request).await.unwrap();
        let csv_text = String::from_utf8_lossy(&report.data);
        assert_eq!(csv_text, "\nEmpty CSV");
    }

    #[tokio::test]
    async fn test_json_generation() {
        let engine = InMemoryReportEngine::new();
        let request = ReportRequest {
            report_type: ReportType::StockSummary,
            format: ReportFormat::Json,
            tenant_id: 1,
            title: "Stock".to_string(),
            parameters: serde_json::json!({
                "warehouse_id": 5,
                "include_zero": false
            }),
            requested_by: None,
            locale: None,
        };

        let report = engine.generate(request).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&report.data).unwrap();
        assert_eq!(json["warehouse_id"], 5);
        assert_eq!(json["include_zero"], false);
    }

    #[tokio::test]
    async fn test_custom_report_type() {
        let engine = InMemoryReportEngine::new();
        let request = ReportRequest {
            report_type: ReportType::Custom("audit_trail".to_string()),
            format: ReportFormat::Pdf,
            tenant_id: 1,
            title: "Custom Report".to_string(),
            parameters: serde_json::json!({}),
            requested_by: None,
            locale: None,
        };

        let report = engine.generate(request).await.unwrap();
        assert!(report.filename.contains("custom_audit_trail"));
        assert!(report.filename.ends_with(".pdf"));
    }

    #[tokio::test]
    async fn test_list_reports_pagination() {
        let engine = InMemoryReportEngine::new();

        for i in 0..5 {
            engine
                .generate(ReportRequest {
                    report_type: ReportType::Invoice,
                    format: ReportFormat::Pdf,
                    tenant_id: 1,
                    title: format!("Report {}", i),
                    parameters: serde_json::json!({}),
                    requested_by: None,
                    locale: None,
                })
                .await
                .unwrap();
        }

        let all = engine.list_reports(1, 10, 0).await.unwrap();
        assert_eq!(all.len(), 5);

        let page1 = engine.list_reports(1, 2, 0).await.unwrap();
        assert_eq!(page1.len(), 2);

        let page2 = engine.list_reports(1, 2, 2).await.unwrap();
        assert_eq!(page2.len(), 2);

        let page3 = engine.list_reports(1, 2, 4).await.unwrap();
        assert_eq!(page3.len(), 1);

        let empty = engine.list_reports(1, 10, 10).await.unwrap();
        assert!(empty.is_empty());
    }

    #[tokio::test]
    async fn test_delete_report_not_found() {
        let engine = InMemoryReportEngine::new();
        let result = engine.delete_report(1, 999).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[test]
    fn test_report_format_content_types() {
        assert_eq!(ReportFormat::Pdf.content_type(), "application/pdf");
        assert_eq!(
            ReportFormat::Excel.content_type(),
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
        );
        assert_eq!(ReportFormat::Xml.content_type(), "application/xml");
        assert_eq!(ReportFormat::Csv.content_type(), "text/csv");
        assert_eq!(ReportFormat::Json.content_type(), "application/json");
    }

    #[test]
    fn test_report_format_extensions() {
        assert_eq!(ReportFormat::Pdf.extension(), "pdf");
        assert_eq!(ReportFormat::Excel.extension(), "xlsx");
        assert_eq!(ReportFormat::Xml.extension(), "xml");
        assert_eq!(ReportFormat::Csv.extension(), "csv");
        assert_eq!(ReportFormat::Json.extension(), "json");
    }

    #[test]
    fn test_report_type_display() {
        assert_eq!(format!("{}", ReportType::Invoice), "invoice");
        assert_eq!(format!("{}", ReportType::TrialBalance), "trial_balance");
        assert_eq!(format!("{}", ReportType::BalanceSheet), "balance_sheet");
        assert_eq!(
            format!("{}", ReportType::IncomeStatement),
            "income_statement"
        );
        assert_eq!(format!("{}", ReportType::PayrollSummary), "payroll_summary");
        assert_eq!(format!("{}", ReportType::StockSummary), "stock_summary");
        assert_eq!(format!("{}", ReportType::SalesReport), "sales_report");
        assert_eq!(format!("{}", ReportType::PurchaseReport), "purchase_report");
        assert_eq!(format!("{}", ReportType::AgingReport), "aging_report");
        assert_eq!(format!("{}", ReportType::EDefter), "edefter");
        assert_eq!(
            format!("{}", ReportType::Custom("special".to_string())),
            "custom_special"
        );
    }

    #[test]
    fn test_report_meta_from_generated() {
        let report = GeneratedReport {
            id: 42,
            report_type: ReportType::Invoice,
            format: ReportFormat::Pdf,
            tenant_id: 1,
            title: "Test".to_string(),
            data: vec![1, 2, 3, 4, 5],
            filename: "invoice_1_42.pdf".to_string(),
            content_type: "application/pdf".to_string(),
            generated_at: Utc::now(),
            generated_by: Some(7),
        };

        let meta = ReportMeta::from(&report);
        assert_eq!(meta.id, 42);
        assert_eq!(meta.size_bytes, 5);
        assert_eq!(meta.report_type, ReportType::Invoice);
        assert_eq!(meta.format, ReportFormat::Pdf);
        assert_eq!(meta.generated_by, Some(7));
    }

    #[tokio::test]
    async fn test_filename_format() {
        let engine = InMemoryReportEngine::new();
        let request = ReportRequest {
            report_type: ReportType::BalanceSheet,
            format: ReportFormat::Excel,
            tenant_id: 99,
            title: "Q1".to_string(),
            parameters: serde_json::json!({}),
            requested_by: None,
            locale: None,
        };

        let report = engine.generate(request).await.unwrap();
        // Expected: balance_sheet_99_1.xlsx
        assert!(report.filename.starts_with("balance_sheet_"));
        assert!(report.filename.ends_with(".xlsx"));
        assert!(report.filename.contains("_99_"));
    }

    #[tokio::test]
    async fn test_generate_all_report_types() {
        let engine = InMemoryReportEngine::new();
        let types = vec![
            ReportType::Invoice,
            ReportType::TrialBalance,
            ReportType::BalanceSheet,
            ReportType::IncomeStatement,
            ReportType::PayrollSummary,
            ReportType::StockSummary,
            ReportType::SalesReport,
            ReportType::PurchaseReport,
            ReportType::AgingReport,
            ReportType::EDefter,
        ];

        for (i, rt) in types.iter().enumerate() {
            let request = ReportRequest {
                report_type: rt.clone(),
                format: ReportFormat::Json,
                tenant_id: 1,
                title: format!("Report {}", i),
                parameters: serde_json::json!({}),
                requested_by: None,
                locale: None,
            };
            let report = engine.generate(request).await.unwrap();
            assert!(!report.data.is_empty());
            assert_eq!(report.id, (i + 1) as i64);
        }

        let meta = engine.list_reports(1, 100, 0).await.unwrap();
        assert_eq!(meta.len(), types.len());
    }

    #[tokio::test]
    async fn test_tenant_isolation_multiple_tenants() {
        let engine = InMemoryReportEngine::new();

        for tenant in [1i64, 2, 3] {
            for _ in 0..3 {
                engine
                    .generate(ReportRequest {
                        report_type: ReportType::Invoice,
                        format: ReportFormat::Pdf,
                        tenant_id: tenant,
                        title: format!("Tenant {} Report", tenant),
                        parameters: serde_json::json!({}),
                        requested_by: None,
                        locale: None,
                    })
                    .await
                    .unwrap();
            }
        }

        for tenant in [1i64, 2, 3] {
            let meta = engine.list_reports(tenant, 10, 0).await.unwrap();
            assert_eq!(meta.len(), 3);
            assert!(meta.iter().all(|m| m.tenant_id == tenant));
        }
    }
}
