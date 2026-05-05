//! Report template definitions

use super::{ReportFormat, ReportType};

/// A report template defines layout and fields for a report type
#[derive(Debug, Clone)]
pub struct ReportTemplate {
    pub id: String,
    pub name: String,
    pub report_type: ReportType,
    pub format: ReportFormat,
    pub headers: Vec<String>,
    pub columns: Vec<String>,
}

impl ReportTemplate {
    pub fn invoice_pdf() -> Self {
        Self {
            id: "invoice_pdf".to_string(),
            name: "Invoice PDF".to_string(),
            report_type: ReportType::Invoice,
            format: ReportFormat::Pdf,
            headers: vec![
                "Invoice No".to_string(),
                "Date".to_string(),
                "Customer".to_string(),
                "Total".to_string(),
            ],
            columns: vec![
                "item_description".to_string(),
                "quantity".to_string(),
                "unit_price".to_string(),
                "line_total".to_string(),
            ],
        }
    }

    pub fn trial_balance_excel() -> Self {
        Self {
            id: "trial_balance_excel".to_string(),
            name: "Trial Balance".to_string(),
            report_type: ReportType::TrialBalance,
            format: ReportFormat::Excel,
            headers: vec![
                "Account Code".to_string(),
                "Account Name".to_string(),
                "Debit".to_string(),
                "Credit".to_string(),
            ],
            columns: vec![
                "account_code".to_string(),
                "account_name".to_string(),
                "debit".to_string(),
                "credit".to_string(),
            ],
        }
    }

    pub fn payroll_excel() -> Self {
        Self {
            id: "payroll_excel".to_string(),
            name: "Payroll Summary".to_string(),
            report_type: ReportType::PayrollSummary,
            format: ReportFormat::Excel,
            headers: vec![
                "Employee".to_string(),
                "Gross Salary".to_string(),
                "Deductions".to_string(),
                "Net Salary".to_string(),
            ],
            columns: vec![
                "employee_name".to_string(),
                "gross".to_string(),
                "deductions".to_string(),
                "net".to_string(),
            ],
        }
    }

    pub fn edefter_xml() -> Self {
        Self {
            id: "edefter_xml".to_string(),
            name: "e-Defter UBL-TR".to_string(),
            report_type: ReportType::EDefter,
            format: ReportFormat::Xml,
            headers: vec![],
            columns: vec![],
        }
    }

    pub fn stock_csv() -> Self {
        Self {
            id: "stock_csv".to_string(),
            name: "Stock Summary CSV".to_string(),
            report_type: ReportType::StockSummary,
            format: ReportFormat::Csv,
            headers: vec![
                "Product Code".to_string(),
                "Product Name".to_string(),
                "Quantity".to_string(),
                "Warehouse".to_string(),
            ],
            columns: vec![
                "product_code".to_string(),
                "product_name".to_string(),
                "quantity".to_string(),
                "warehouse".to_string(),
            ],
        }
    }
}
