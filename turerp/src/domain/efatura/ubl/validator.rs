//! UBL-TR XML structural validator for e-Fatura
//!
//! Provides basic validation of UBL-TR XML documents and EFatura domain
//! objects before XML generation. This is not a full XSD schema validation;
//! it checks structural requirements mandated by the GIB specification.

use chrono::Utc;
use rust_decimal::Decimal;

use crate::domain::efatura::model::{EFatura, EFaturaLine, EFaturaProfile, MonetaryTotal};

/// Result of a validation pass.
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl ValidationResult {
    /// Create a successful (empty) validation result.
    pub fn ok() -> Self {
        ValidationResult {
            is_valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Create a failed validation result with a single error.
    pub fn error(msg: impl Into<String>) -> Self {
        ValidationResult {
            is_valid: false,
            errors: vec![msg.into()],
            warnings: Vec::new(),
        }
    }

    /// Merge another validation result into this one.
    pub fn merge(&mut self, other: &ValidationResult) {
        self.errors.extend(other.errors.iter().cloned());
        self.warnings.extend(other.warnings.iter().cloned());
        if !other.is_valid {
            self.is_valid = false;
        }
    }

    /// Add an error and mark the result as invalid.
    pub fn add_error(&mut self, msg: impl Into<String>) {
        self.errors.push(msg.into());
        self.is_valid = false;
    }

    /// Add a warning (does not affect validity).
    pub fn add_warning(&mut self, msg: impl Into<String>) {
        self.warnings.push(msg.into());
    }
}

// ---------------------------------------------------------------------------
// VKN / TCKN validation
// ---------------------------------------------------------------------------

/// Validate a VKN (10 digits) or TCKN (11 digits) identifier.
fn validate_vkn_tckn(vkn_tckn: &str, label: &str, errors: &mut Vec<String>) {
    if vkn_tckn.is_empty() {
        errors.push(format!("{} VKN/TCKN must not be empty", label));
        return;
    }

    if !vkn_tckn.chars().all(|c| c.is_ascii_digit()) {
        errors.push(format!(
            "{} VKN/TCKN must contain only digits, got: {}",
            label, vkn_tckn
        ));
        return;
    }

    let len = vkn_tckn.len();
    if len != 10 && len != 11 {
        errors.push(format!(
            "{} VKN/TCKN must be 10 (VKN) or 11 (TCKN) digits, got {} digits",
            label, len
        ));
    }
}

// ---------------------------------------------------------------------------
// Document number validation
// ---------------------------------------------------------------------------

/// Validate a document number format.
///
/// GIB document numbers follow the pattern: 3 letters + year (4 digits) +
/// running number (9 digits). Common formats:
/// - `ABC2024000000001` (prefix + year + 9 digits)
/// - Also accepts simpler formats for testing.
fn validate_document_number(doc_number: &str, errors: &mut Vec<String>) {
    if doc_number.is_empty() {
        errors.push("document_number must not be empty".to_string());
        return;
    }

    // Minimum: at least 3 characters
    if doc_number.len() < 3 {
        errors.push(format!(
            "document_number must be at least 3 characters, got: {}",
            doc_number
        ));
        return;
    }

    // The first 3 characters should be alphabetic (prefix)
    let prefix = &doc_number[..3];
    if !prefix.chars().all(|c| c.is_ascii_alphabetic()) {
        errors.push(format!(
            "document_number must start with 3 alphabetic characters, got prefix: {}",
            prefix
        ));
    }

    // After the prefix, there should be digits (year + running number)
    if doc_number.len() > 3 {
        let suffix = &doc_number[3..];
        if !suffix
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            errors.push(format!(
                "document_number suffix must be alphanumeric (dashes/underscores allowed), got: {}",
                suffix
            ));
        }
    }
}

// ---------------------------------------------------------------------------
// EFatura domain validation
// ---------------------------------------------------------------------------

/// Validate an EFatura domain object before XML generation.
///
/// Checks:
/// - UUID and document_number are present
/// - VKN/TCKN for sender and receiver are valid (10 or 11 digits)
/// - Document number format is correct
/// - Lines are not empty
/// - Monetary totals are non-negative
/// - Issue date is not in the future
pub fn validate_efatura(fatura: &EFatura) -> ValidationResult {
    let mut result = ValidationResult::ok();
    let mut errors = Vec::new();

    // UUID
    if fatura.uuid.is_empty() {
        errors.push("uuid must not be empty".to_string());
    }

    // Document number
    validate_document_number(&fatura.document_number, &mut errors);

    // VKN/TCKN
    validate_vkn_tckn(&fatura.sender.vkn_tckn, "Sender", &mut errors);
    validate_vkn_tckn(&fatura.receiver.vkn_tckn, "Receiver", &mut errors);

    // Party names
    if fatura.sender.name.is_empty() {
        errors.push("sender name must not be empty".to_string());
    }
    if fatura.receiver.name.is_empty() {
        errors.push("receiver name must not be empty".to_string());
    }

    // Lines
    if fatura.lines.is_empty() {
        result.add_warning("invoice has no line items".to_string());
    } else {
        for (i, line) in fatura.lines.iter().enumerate() {
            validate_line(line, i, &mut errors);
        }
    }

    // Monetary totals
    validate_monetary_total(&fatura.legal_monetary_total, &mut errors);

    // Tax totals
    for (i, tax) in fatura.tax_totals.iter().enumerate() {
        if tax.tax_amount < Decimal::ZERO {
            errors.push(format!(
                "tax_total[{}] tax_amount must be non-negative, got {}",
                i, tax.tax_amount
            ));
        }
        if tax.rate < Decimal::ZERO {
            errors.push(format!(
                "tax_total[{}] rate must be non-negative, got {}",
                i, tax.rate
            ));
        }
    }

    // Issue date should not be in the future
    let today = Utc::now().date_naive();
    if fatura.issue_date > today {
        errors.push(format!(
            "issue_date {} is in the future (today: {})",
            fatura.issue_date, today
        ));
    }

    // Profile ID
    match &fatura.profile_id {
        EFaturaProfile::TemelFatura
        | EFaturaProfile::Ihracat
        | EFaturaProfile::YolcuBeleni
        | EFaturaProfile::OzelMatbuFatura => {} // valid
    }

    for err in errors {
        result.add_error(err);
    }

    result
}

fn validate_line(line: &EFaturaLine, index: usize, errors: &mut Vec<String>) {
    if line.id.is_empty() {
        errors.push(format!("line[{}] id must not be empty", index));
    }
    if line.product_name.is_empty() {
        errors.push(format!("line[{}] product_name must not be empty", index));
    }
    if line.quantity < Decimal::ZERO {
        errors.push(format!("line[{}] quantity must be non-negative", index));
    }
    if line.unit_price < Decimal::ZERO {
        errors.push(format!("line[{}] unit_price must be non-negative", index));
    }
    if line.line_amount < Decimal::ZERO {
        errors.push(format!("line[{}] line_amount must be non-negative", index));
    }
    if line.tax_rate < Decimal::ZERO {
        errors.push(format!("line[{}] tax_rate must be non-negative", index));
    }
    if line.tax_amount < Decimal::ZERO {
        errors.push(format!("line[{}] tax_amount must be non-negative", index));
    }
}

fn validate_monetary_total(total: &MonetaryTotal, errors: &mut Vec<String>) {
    if total.line_extension_amount < Decimal::ZERO {
        errors.push(format!(
            "line_extension_amount must be non-negative, got {}",
            total.line_extension_amount
        ));
    }
    if total.tax_exclusive_amount < Decimal::ZERO {
        errors.push(format!(
            "tax_exclusive_amount must be non-negative, got {}",
            total.tax_exclusive_amount
        ));
    }
    if total.tax_inclusive_amount < Decimal::ZERO {
        errors.push(format!(
            "tax_inclusive_amount must be non-negative, got {}",
            total.tax_inclusive_amount
        ));
    }
    if total.payable_amount < Decimal::ZERO {
        errors.push(format!(
            "payable_amount must be non-negative, got {}",
            total.payable_amount
        ));
    }
    if let Some(allowance) = total.allowance_total_amount {
        if allowance < Decimal::ZERO {
            errors.push(format!(
                "allowance_total_amount must be non-negative, got {}",
                allowance
            ));
        }
    }
}

// ---------------------------------------------------------------------------
// UBL-TR XML structural validation
// ---------------------------------------------------------------------------

/// Validate a UBL-TR XML document's structural integrity.
///
/// This performs basic checks that the XML contains the required elements
/// and that they are well-formed. It is NOT a full XSD schema validation.
///
/// Checks:
/// - Invoice root element present
/// - UBLVersionID present and equals "2.1"
/// - ID (document number) present
/// - IssueDate present
/// - AccountingSupplierParty present with VKN and Name
/// - AccountingCustomerParty present with VKN and Name
/// - VKN/TCKN values are 10 or 11 digits
/// - Document number format is valid
/// - InvoiceLine elements exist (at least one)
/// - Monetary amounts are non-negative
/// - Issue date is not in the future
pub fn validate_ubl_xml(xml: &str) -> ValidationResult {
    let mut result = ValidationResult::ok();
    let mut errors = Vec::new();

    // Empty check
    if xml.trim().is_empty() {
        result.add_error("XML document is empty");
        return result;
    }

    // Invoice root element
    if !xml.contains("<Invoice") {
        result.add_error("XML does not contain an Invoice root element");
        return result;
    }

    // Closing tag
    if !xml.contains("</Invoice>") {
        result.add_warning("Invoice element is not closed");
    }

    // UBLVersionID
    match extract_tag_value(xml, "cbc:UBLVersionID") {
        Some(version) => {
            if version != "2.1" {
                errors.push(format!("UBLVersionID must be '2.1', got '{}'", version));
            }
        }
        None => errors.push("Missing required element: cbc:UBLVersionID".to_string()),
    }

    // ID (document number)
    let doc_number = extract_tag_value(xml, "cbc:ID");
    match &doc_number {
        Some(num) => validate_document_number(num, &mut errors),
        None => errors.push("Missing required element: cbc:ID (document number)".to_string()),
    }

    // UUID
    if extract_tag_value(xml, "cbc:UUID").is_none() {
        errors.push("Missing required element: cbc:UUID".to_string());
    }

    // IssueDate
    match extract_tag_value(xml, "cbc:IssueDate") {
        Some(date_str) => {
            // Check date is not in the future
            if let Ok(date) = chrono::NaiveDate::parse_from_str(&date_str, "%Y-%m-%d") {
                let today = Utc::now().date_naive();
                if date > today {
                    errors.push(format!(
                        "IssueDate {} is in the future (today: {})",
                        date_str, today
                    ));
                }
            } else {
                errors.push(format!(
                    "IssueDate has invalid format, expected YYYY-MM-DD: {}",
                    date_str
                ));
            }
        }
        None => errors.push("Missing required element: cbc:IssueDate".to_string()),
    }

    // InvoiceTypeCode
    if extract_tag_value(xml, "cbc:InvoiceTypeCode").is_none() {
        result.add_warning("Missing optional element: cbc:InvoiceTypeCode".to_string());
    }

    // AccountingSupplierParty
    let sender_vkn = extract_nested_value(xml, "AccountingSupplierParty", "cbc:VKN")
        .or_else(|| extract_nested_value(xml, "AccountingSupplierParty", "cbc:TCKN"));
    match &sender_vkn {
        Some(vkn) => validate_vkn_tckn(vkn, "Sender", &mut errors),
        None => errors.push("Missing VKN/TCKN in AccountingSupplierParty".to_string()),
    }

    let sender_name = extract_nested_value(xml, "AccountingSupplierParty", "cbc:Name");
    if sender_name.is_none() || sender_name.as_ref().is_none_or(|n| n.is_empty()) {
        errors.push("Missing or empty Name in AccountingSupplierParty".to_string());
    }

    // AccountingCustomerParty
    let receiver_vkn = extract_nested_value(xml, "AccountingCustomerParty", "cbc:VKN")
        .or_else(|| extract_nested_value(xml, "AccountingCustomerParty", "cbc:TCKN"));
    match &receiver_vkn {
        Some(vkn) => validate_vkn_tckn(vkn, "Receiver", &mut errors),
        None => errors.push("Missing VKN/TCKN in AccountingCustomerParty".to_string()),
    }

    let receiver_name = extract_nested_value(xml, "AccountingCustomerParty", "cbc:Name");
    if receiver_name.is_none() || receiver_name.as_ref().is_none_or(|n| n.is_empty()) {
        errors.push("Missing or empty Name in AccountingCustomerParty".to_string());
    }

    // Invoice lines
    let line_count = xml.matches("<cac:InvoiceLine").count();
    if line_count == 0 {
        result.add_warning("Invoice has no InvoiceLine elements".to_string());
    }

    // Monetary amounts (non-negative)
    if let Some(amount_str) = extract_tag_value(xml, "cbc:TaxInclusiveAmount") {
        if let Ok(amount) = amount_str.parse::<Decimal>() {
            if amount < Decimal::ZERO {
                errors.push(format!(
                    "TaxInclusiveAmount must be non-negative, got {}",
                    amount
                ));
            }
        }
    }

    if let Some(amount_str) = extract_tag_value(xml, "cbc:PayableAmount") {
        if let Ok(amount) = amount_str.parse::<Decimal>() {
            if amount < Decimal::ZERO {
                errors.push(format!(
                    "PayableAmount must be non-negative, got {}",
                    amount
                ));
            }
        }
    }

    for err in errors {
        result.add_error(err);
    }

    result
}

// ---------------------------------------------------------------------------
// Internal helpers (shared with mapper)
// ---------------------------------------------------------------------------

/// Extract the text content of the first occurrence of a simple XML tag.
///
/// Handles both `<cbc:ID>value</cbc:ID>` and
/// `<cbc:Amount currencyID="TRY">value</cbc:Amount>` (tags with attributes).
fn extract_tag_value(xml: &str, tag: &str) -> Option<String> {
    let close = format!("</{tag}>");

    // Try exact match first (no attributes)
    let open_exact = format!("<{tag}>");
    if let Some(start) = xml.find(&open_exact) {
        let content_start = start + open_exact.len();
        if let Some(end) = xml[content_start..].find(&close) {
            return Some(xml[content_start..content_start + end].to_string());
        }
    }

    // Try match with attributes: <tag ...>
    let open_prefix = format!("<{tag} ");
    if let Some(start) = xml.find(&open_prefix) {
        let tag_body_start = start + open_prefix.len();
        if let Some(tag_end) = xml[tag_body_start..].find('>') {
            let content_start = tag_body_start + tag_end + 1;
            if let Some(end) = xml[content_start..].find(&close) {
                return Some(xml[content_start..content_start + end].to_string());
            }
        }
    }

    None
}

/// Extract the text content of a tag within a named parent section.
fn extract_nested_value(xml: &str, parent: &str, tag: &str) -> Option<String> {
    let parent_open = format!("<cac:{parent}>");
    let parent_close = format!("</cac:{parent}>");

    let parent_start = xml.find(&parent_open)?;
    let parent_content_start = parent_start + parent_open.len();
    let parent_end = xml[parent_content_start..].find(&parent_close)?;
    let parent_section = &xml[parent_content_start..parent_content_start + parent_end];

    extract_tag_value(parent_section, tag)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::efatura::model::{
        AddressInfo, EFaturaLine, EFaturaStatus, MonetaryTotal, PartyInfo, TaxSubtotal,
    };
    use crate::domain::efatura::ubl::mapper::efatura_to_ubl_xml;
    use chrono::NaiveDate;

    fn sample_fatura() -> EFatura {
        EFatura {
            id: 1,
            tenant_id: 100,
            invoice_id: Some(42),
            uuid: "3f2504e0-4f89-11d3-9a0c-0305e82c3301".to_string(),
            document_number: "ABC2024000000001".to_string(),
            issue_date: NaiveDate::from_ymd_opt(2024, 6, 15).unwrap(),
            profile_id: EFaturaProfile::TemelFatura,
            sender: PartyInfo {
                vkn_tckn: "1234567890".to_string(),
                name: "Acme Corp".to_string(),
                tax_office: "Kadikoy".to_string(),
                address: AddressInfo {
                    street: "Main St 1".to_string(),
                    district: Some("Kadikoy".to_string()),
                    city: "Istanbul".to_string(),
                    country: Some("Turkey".to_string()),
                    postal_code: Some("34700".to_string()),
                },
                email: Some("info@acme.com".to_string()),
                phone: None,
                register_number: None,
                mersis_number: None,
            },
            receiver: PartyInfo {
                vkn_tckn: "9876543210".to_string(),
                name: "Buyer Ltd".to_string(),
                tax_office: "Uskudar".to_string(),
                address: AddressInfo {
                    street: "Side St 2".to_string(),
                    district: None,
                    city: "Istanbul".to_string(),
                    country: None,
                    postal_code: None,
                },
                email: None,
                phone: None,
                register_number: None,
                mersis_number: None,
            },
            lines: vec![EFaturaLine {
                id: "1".to_string(),
                product_name: "Widget A".to_string(),
                quantity: Decimal::new(10, 0),
                unit: "C62".to_string(),
                unit_price: Decimal::new(1000, 2),
                line_amount: Decimal::new(10000, 2),
                tax_rate: Decimal::new(18, 1),
                tax_amount: Decimal::new(1800, 2),
            }],
            tax_totals: vec![TaxSubtotal {
                tax_type: "VAT".to_string(),
                taxable_amount: Decimal::new(10000, 2),
                tax_amount: Decimal::new(1800, 2),
                rate: Decimal::new(18, 1),
            }],
            legal_monetary_total: MonetaryTotal {
                line_extension_amount: Decimal::new(10000, 2),
                tax_exclusive_amount: Decimal::new(10000, 2),
                tax_inclusive_amount: Decimal::new(11800, 2),
                allowance_total_amount: None,
                payable_amount: Decimal::new(11800, 2),
            },
            status: EFaturaStatus::Draft,
            response_code: None,
            response_desc: None,
            xml_content: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn test_validate_valid_xml() {
        let fatura = sample_fatura();
        let xml = efatura_to_ubl_xml(&fatura).unwrap();
        let result = validate_ubl_xml(&xml);
        assert!(
            result.is_valid,
            "Expected valid XML, errors: {:?}",
            result.errors
        );
    }

    #[test]
    fn test_validate_missing_fields() {
        // Missing required fields
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Invoice xmlns="urn:oasis:names:specification:ubl:schema:xsd:Invoice-2"
          xmlns:cac="urn:oasis:names:specification:ubl:schema:xsd:CommonAggregateComponents-2"
          xmlns:cbc="urn:oasis:names:specification:ubl:schema:xsd:CommonBasicComponents-2">
  <cbc:UBLVersionID>2.1</cbc:UBLVersionID>
  <cac:AccountingSupplierParty>
    <cac:Party>
      <cbc:VKN>1234567890</cbc:VKN>
      <cbc:Name>Sender</cbc:Name>
    </cac:Party>
  </cac:AccountingSupplierParty>
  <cac:AccountingCustomerParty>
    <cac:Party>
      <cbc:VKN>9876543210</cbc:VKN>
      <cbc:Name>Receiver</cbc:Name>
    </cac:Party>
  </cac:AccountingCustomerParty>
</Invoice>"#;

        let result = validate_ubl_xml(xml);
        assert!(!result.is_valid, "Expected invalid XML, but it passed");

        // Should have errors for missing ID, UUID, IssueDate
        assert!(
            result.errors.iter().any(|e| e.contains("cbc:ID")),
            "Expected error about missing ID, got: {:?}",
            result.errors
        );
        assert!(
            result.errors.iter().any(|e| e.contains("cbc:UUID")),
            "Expected error about missing UUID, got: {:?}",
            result.errors
        );
        assert!(
            result.errors.iter().any(|e| e.contains("cbc:IssueDate")),
            "Expected error about missing IssueDate, got: {:?}",
            result.errors
        );
    }

    #[test]
    fn test_validate_invalid_vkn() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Invoice xmlns="urn:oasis:names:specification:ubl:schema:xsd:Invoice-2"
          xmlns:cac="urn:oasis:names:specification:ubl:schema:xsd:CommonAggregateComponents-2"
          xmlns:cbc="urn:oasis:names:specification:ubl:schema:xsd:CommonBasicComponents-2">
  <cbc:UBLVersionID>2.1</cbc:UBLVersionID>
  <cbc:ID>ABC2024000000001</cbc:ID>
  <cbc:UUID>test-uuid</cbc:UUID>
  <cbc:IssueDate>2024-01-15</cbc:IssueDate>
  <cac:AccountingSupplierParty>
    <cac:Party>
      <cbc:VKN>123</cbc:VKN>
      <cbc:Name>Sender</cbc:Name>
    </cac:Party>
  </cac:AccountingSupplierParty>
  <cac:AccountingCustomerParty>
    <cac:Party>
      <cbc:VKN>9876543210</cbc:VKN>
      <cbc:Name>Receiver</cbc:Name>
    </cac:Party>
  </cac:AccountingCustomerParty>
</Invoice>"#;

        let result = validate_ubl_xml(xml);
        assert!(!result.is_valid);
        assert!(result
            .errors
            .iter()
            .any(|e| e.contains("VKN") && e.contains("10")));
    }

    #[test]
    fn test_validate_empty_lines() {
        let mut fatura = sample_fatura();
        fatura.lines = vec![];
        let result = validate_efatura(&fatura);
        // Empty lines is a warning, not an error
        assert!(result.warnings.iter().any(|w| w.contains("line items")));
    }

    #[test]
    fn test_validate_efatura_valid() {
        let fatura = sample_fatura();
        let result = validate_efatura(&fatura);
        assert!(
            result.is_valid,
            "Expected valid EFatura, errors: {:?}",
            result.errors
        );
    }

    #[test]
    fn test_validate_efatura_invalid_vkn() {
        let mut fatura = sample_fatura();
        fatura.sender.vkn_tckn = "12345".to_string(); // 5 digits - invalid
        let result = validate_efatura(&fatura);
        assert!(!result.is_valid);
        assert!(result
            .errors
            .iter()
            .any(|e| e.contains("VKN") && e.contains("5")));
    }

    #[test]
    fn test_validate_efatura_negative_amounts() {
        let mut fatura = sample_fatura();
        fatura.legal_monetary_total.payable_amount = Decimal::new(-500, 0);
        let result = validate_efatura(&fatura);
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.contains("payable_amount")));
    }

    #[test]
    fn test_validate_efatura_empty_sender_name() {
        let mut fatura = sample_fatura();
        fatura.sender.name = String::new();
        let result = validate_efatura(&fatura);
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.contains("sender name")));
    }

    #[test]
    fn test_validate_ubl_xml_no_invoice_root() {
        let xml = "<NotInvoice><cbc:ID>ABC</cbc:ID></NotInvoice>";
        let result = validate_ubl_xml(xml);
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.contains("Invoice")));
    }

    #[test]
    fn test_validate_ubl_xml_future_date() {
        let mut fatura = sample_fatura();
        // Set date far in the future
        fatura.issue_date = NaiveDate::from_ymd_opt(2099, 1, 1).unwrap();
        let xml = efatura_to_ubl_xml(&fatura).unwrap();
        let result = validate_ubl_xml(&xml);
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.contains("future")));
    }

    #[test]
    fn test_validation_result_merge() {
        let mut r1 = ValidationResult::ok();
        let r2 = ValidationResult::error("test error");
        r1.merge(&r2);
        assert!(!r1.is_valid);
        assert_eq!(r1.errors.len(), 1);
    }

    #[test]
    fn test_validation_result_add_warning() {
        let mut r = ValidationResult::ok();
        r.add_warning("test warning");
        assert!(r.is_valid); // warnings don't invalidate
        assert_eq!(r.warnings.len(), 1);
    }

    #[test]
    fn test_vkn_tckn_validation() {
        let mut errors = Vec::new();

        // Valid VKN (10 digits)
        validate_vkn_tckn("1234567890", "Test", &mut errors);
        assert!(errors.is_empty());

        // Valid TCKN (11 digits)
        validate_vkn_tckn("12345678901", "Test", &mut errors);
        assert!(errors.is_empty());

        // Invalid: 5 digits
        validate_vkn_tckn("12345", "Test", &mut errors);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("5 digits"));
        errors.clear();

        // Invalid: non-numeric
        validate_vkn_tckn("ABCDEFGHIJ", "Test", &mut errors);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("only digits"));
        errors.clear();

        // Invalid: empty
        validate_vkn_tckn("", "Test", &mut errors);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("empty"));
    }

    #[test]
    fn test_document_number_validation() {
        let mut errors = Vec::new();

        // Valid
        validate_document_number("ABC2024000000001", &mut errors);
        assert!(errors.is_empty());

        // Invalid: starts with digits
        validate_document_number("123ABC", &mut errors);
        assert!(!errors.is_empty());
        errors.clear();

        // Invalid: too short
        validate_document_number("AB", &mut errors);
        assert!(!errors.is_empty());
        errors.clear();

        // Invalid: empty
        validate_document_number("", &mut errors);
        assert!(!errors.is_empty());
    }
}
