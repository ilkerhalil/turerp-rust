//! In-memory report engine implementation

use chrono::Utc;
use parking_lot::RwLock;
use std::collections::HashMap;

use super::{csv, excel, pdf, xml};
use super::{GeneratedReport, ReportEngine, ReportError, ReportFormat, ReportMeta, ReportRequest};

/// In-memory report engine with real format generation
pub struct InMemoryReportEngine {
    reports: RwLock<Vec<GeneratedReport>>,
    next_id: RwLock<i64>,
    job_reports: RwLock<HashMap<i64, i64>>,
}

impl InMemoryReportEngine {
    pub fn new() -> Self {
        Self {
            reports: RwLock::new(Vec::new()),
            next_id: RwLock::new(1),
            job_reports: RwLock::new(HashMap::new()),
        }
    }

    fn allocate_id(&self) -> i64 {
        let mut id = self.next_id.write();
        let report_id = *id;
        *id += 1;
        report_id
    }

    fn generate_data(request: &ReportRequest) -> Result<Vec<u8>, ReportError> {
        match request.format {
            ReportFormat::Pdf => pdf::generate_invoice_pdf(request),
            ReportFormat::Excel => excel::generate_excel(request),
            ReportFormat::Xml => xml::generate_edefter_xml(request),
            ReportFormat::Csv => csv::generate_csv(request),
            ReportFormat::Json => Ok(serde_json::to_string_pretty(&request.parameters)
                .unwrap_or_else(|_| "{}".to_string())
                .into_bytes()),
        }
    }
}

impl Default for InMemoryReportEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl ReportEngine for InMemoryReportEngine {
    async fn generate(&self, request: ReportRequest) -> Result<GeneratedReport, ReportError> {
        let id = self.allocate_id();
        let data = Self::generate_data(&request)?;
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
    ) -> Result<Option<GeneratedReport>, ReportError> {
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
    ) -> Result<Vec<ReportMeta>, ReportError> {
        let reports = self.reports.read();
        Ok(reports
            .iter()
            .filter(|r| r.tenant_id == tenant_id)
            .skip(offset as usize)
            .take(limit as usize)
            .map(ReportMeta::from)
            .collect())
    }

    async fn delete_report(&self, tenant_id: i64, report_id: i64) -> Result<(), ReportError> {
        let mut reports = self.reports.write();
        let idx = reports
            .iter()
            .position(|r| r.id == report_id && r.tenant_id == tenant_id)
            .ok_or(ReportError::NotFound(report_id))?;
        reports.remove(idx);
        Ok(())
    }

    async fn store_job_mapping(&self, job_id: i64, report_id: i64) -> Result<(), ReportError> {
        self.job_reports.write().insert(job_id, report_id);
        Ok(())
    }

    async fn get_report_for_job(&self, job_id: i64) -> Result<Option<i64>, ReportError> {
        Ok(self.job_reports.read().get(&job_id).copied())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::ReportType;

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
                "customer": "Acme Corp",
                "total": 1500.50,
                "items": [
                    {"description": "Widget", "quantity": 2, "price": 100.0, "total": 200.0}
                ]
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
        assert!(report.data.starts_with(b"%PDF"));
    }

    #[tokio::test]
    async fn test_generate_edefter_xml() {
        let engine = InMemoryReportEngine::new();
        let request = ReportRequest {
            report_type: ReportType::EDefter,
            format: ReportFormat::Xml,
            tenant_id: 1,
            title: "e-Defter 2026-01".to_string(),
            parameters: serde_json::json!({
                "period": "2026-01",
                "entries": [
                    {"date": "2026-01-15", "account_code": "100", "description": "Cash", "debit": 1000.0, "credit": 0.0}
                ]
            }),
            requested_by: Some(1),
            locale: Some("tr".to_string()),
        };

        let report = engine.generate(request).await.unwrap();
        assert_eq!(report.format, ReportFormat::Xml);
        assert!(report.filename.ends_with(".xml"));
        let xml = String::from_utf8(report.data).unwrap();
        assert!(xml.contains("GenericAccountingPacket"));
        assert!(xml.contains("100"));
    }

    #[tokio::test]
    async fn test_generate_excel() {
        let engine = InMemoryReportEngine::new();
        let request = ReportRequest {
            report_type: ReportType::TrialBalance,
            format: ReportFormat::Excel,
            tenant_id: 1,
            title: "Trial Balance".to_string(),
            parameters: serde_json::json!({
                "headers": ["Account", "Debit", "Credit"],
                "rows": [
                    ["100 - Cash", "1000.00", "0.00"],
                    ["200 - Revenue", "0.00", "1000.00"]
                ]
            }),
            requested_by: None,
            locale: None,
        };

        let report = engine.generate(request).await.unwrap();
        assert_eq!(report.format, ReportFormat::Excel);
        assert!(report.filename.ends_with(".xlsx"));
        assert!(report.data.len() > 100);
        assert!(report.data.starts_with(b"PK"));
    }

    #[tokio::test]
    async fn test_generate_csv() {
        let engine = InMemoryReportEngine::new();
        let request = ReportRequest {
            report_type: ReportType::StockSummary,
            format: ReportFormat::Csv,
            tenant_id: 1,
            title: "Stock Summary".to_string(),
            parameters: serde_json::json!({
                "headers": ["Code", "Name", "Qty"],
                "rows": [
                    ["P001", "Widget", "100"],
                    ["P002", "Gadget", "50"]
                ]
            }),
            requested_by: None,
            locale: None,
        };

        let report = engine.generate(request).await.unwrap();
        assert_eq!(report.format, ReportFormat::Csv);
        assert!(report.filename.ends_with(".csv"));
        let csv_text = String::from_utf8(report.data).unwrap();
        assert!(csv_text.contains("P001"));
        assert!(csv_text.contains("Widget"));
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
    async fn test_job_report_mapping() {
        let engine = InMemoryReportEngine::new();
        engine.store_job_mapping(1, 42).await.unwrap();
        assert_eq!(engine.get_report_for_job(1).await.unwrap(), Some(42));
        assert_eq!(engine.get_report_for_job(2).await.unwrap(), None);
    }
}
