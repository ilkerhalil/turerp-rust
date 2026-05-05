//! UBL-TR XML mapper for e-Fatura
//!
//! Converts between EFatura domain objects and UBL-TR XML strings.
//! UBL-TR is Turkey's e-invoicing format based on UBL 2.1.

use rust_decimal::Decimal;

use crate::domain::efatura::model::{
    AddressInfo, EFatura, EFaturaLine, EFaturaProfile, MonetaryTotal, PartyInfo, TaxSubtotal,
};

/// Simplified invoice representation extracted from UBL XML.
///
/// This is a partial parse -- it extracts key header fields but does not
/// attempt to reconstruct the full EFatura struct (which requires DB ids,
/// timestamps, etc. that are not in the XML).
#[derive(Debug, Clone)]
pub struct UblPartialInvoice {
    pub document_number: String,
    pub uuid: String,
    pub issue_date: String,
    pub sender_vkn: String,
    pub sender_name: String,
    pub receiver_vkn: String,
    pub receiver_name: String,
    pub line_count: usize,
    pub tax_inclusive_amount: Decimal,
}

// ---------------------------------------------------------------------------
// XML escape helper
// ---------------------------------------------------------------------------

/// Escape special XML characters in a string.
fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

// ---------------------------------------------------------------------------
// UBL-TR namespace constants
// ---------------------------------------------------------------------------

const NS_INVOICE: &str = "urn:oasis:names:specification:ubl:schema:xsd:Invoice-2";
const NS_CAC: &str = "urn:oasis:names:specification:ubl:schema:xsd:CommonAggregateComponents-2";
const NS_CBC: &str = "urn:oasis:names:specification:ubl:schema:xsd:CommonBasicComponents-2";

// ---------------------------------------------------------------------------
// InvoiceTypeCode mapping
// ---------------------------------------------------------------------------

/// Map an EFaturaProfile to a UBL InvoiceTypeCode.
///
/// Common codes:
/// - 381: Temel Fatura (commercial invoice)
/// - 382: Ihracat (export invoice)
/// - 383: Yolcu Beleni (passenger receipt)
fn profile_to_type_code(profile: &EFaturaProfile) -> &'static str {
    match profile {
        EFaturaProfile::TemelFatura => "381",
        EFaturaProfile::Ihracat => "382",
        EFaturaProfile::YolcuBeleni => "383",
        EFaturaProfile::OzelMatbuFatura => "381",
    }
}

// ---------------------------------------------------------------------------
// Party XML generation
// ---------------------------------------------------------------------------

fn format_party(party: &PartyInfo, tag: &str) -> String {
    let vkn_tckn = xml_escape(&party.vkn_tckn);
    let name = xml_escape(&party.name);
    let tax_office = xml_escape(&party.tax_office);

    let mut xml = format!(
        "    <cac:{tag}>\n\
         <cac:Party>\n\
         <cbc:VKN>{vkn_tckn}</cbc:VKN>\n\
         <cbc:Name>{name}</cbc:Name>\n"
    );

    // Tax office
    xml.push_str(&format!(
        "      <cac:PartyTaxScheme>\n\
         <cbc:Name>{tax_office}</cbc:Name>\n\
         </cac:PartyTaxScheme>\n"
    ));

    // Postal address
    xml.push_str(&format_address(&party.address));

    xml.push_str("    </cac:Party>\n");
    xml.push_str(&format!("  </cac:{tag}>\n"));
    xml
}

fn format_address(addr: &AddressInfo) -> String {
    let street = xml_escape(&addr.street);
    let city = xml_escape(&addr.city);

    let mut xml = String::from("      <cac:PostalAddress>\n");
    xml.push_str(&format!(
        "        <cbc:StreetName>{street}</cbc:StreetName>\n"
    ));

    if let Some(ref district) = addr.district {
        xml.push_str(&format!(
            "        <cbc:District>{}</cbc:District>\n",
            xml_escape(district)
        ));
    }

    xml.push_str(&format!("        <cbc:CityName>{city}</cbc:CityName>\n"));

    if let Some(ref country) = addr.country {
        xml.push_str(&format!(
            "        <cbc:Country>{}</cbc:Country>\n",
            xml_escape(country)
        ));
    }

    if let Some(ref postal) = addr.postal_code {
        xml.push_str(&format!(
            "        <cbc:PostalZone>{}</cbc:PostalZone>\n",
            xml_escape(postal)
        ));
    }

    xml.push_str("      </cac:PostalAddress>\n");
    xml
}

// ---------------------------------------------------------------------------
// Line items
// ---------------------------------------------------------------------------

fn format_line(line: &EFaturaLine, _index: usize) -> String {
    let id = xml_escape(&line.id);
    let product_name = xml_escape(&line.product_name);
    let quantity = line.quantity;
    let unit = xml_escape(&line.unit);
    let unit_price = line.unit_price;
    let line_amount = line.line_amount;
    let tax_rate = line.tax_rate;
    let tax_amount = line.tax_amount;

    format!(
        "  <cac:InvoiceLine>\n\
         <cbc:ID>{id}</cbc:ID>\n\
         <cbc:InvoicedQuantity unitCode=\"{unit}\">{quantity}</cbc:InvoicedQuantity>\n\
         <cbc:LineExtensionAmount currencyID=\"TRY\">{line_amount}</cbc:LineExtensionAmount>\n\
         <cac:Item>\n\
           <cbc:Name>{product_name}</cbc:Name>\n\
         </cac:Item>\n\
         <cac:Price>\n\
           <cbc:PriceAmount currencyID=\"TRY\">{unit_price}</cbc:PriceAmount>\n\
         </cac:Price>\n\
         <cac:TaxTotal>\n\
           <cbc:TaxAmount currencyID=\"TRY\">{tax_amount}</cbc:TaxAmount>\n\
           <cac:TaxSubtotal>\n\
             <cbc:Percent>{tax_rate}</cbc:Percent>\n\
             <cbc:TaxAmount currencyID=\"TRY\">{tax_amount}</cbc:TaxAmount>\n\
           </cac:TaxSubtotal>\n\
         </cac:TaxTotal>\n\
         </cac:InvoiceLine>\n"
    )
}

// ---------------------------------------------------------------------------
// Tax totals
// ---------------------------------------------------------------------------

fn format_tax_total(subtotal: &TaxSubtotal) -> String {
    let tax_type = xml_escape(&subtotal.tax_type);
    let taxable = subtotal.taxable_amount;
    let amount = subtotal.tax_amount;
    let rate = subtotal.rate;

    format!(
        "    <cac:TaxSubtotal>\n\
         <cbc:TaxableAmount currencyID=\"TRY\">{taxable}</cbc:TaxableAmount>\n\
         <cbc:TaxAmount currencyID=\"TRY\">{amount}</cbc:TaxAmount>\n\
         <cbc:Percent>{rate}</cbc:Percent>\n\
         <cac:TaxCategory>\n\
           <cbc:ID>{tax_type}</cbc:ID>\n\
         </cac:TaxCategory>\n\
         </cac:TaxSubtotal>\n"
    )
}

// ---------------------------------------------------------------------------
// Monetary total
// ---------------------------------------------------------------------------

fn format_monetary_total(total: &MonetaryTotal) -> String {
    let line_ext = total.line_extension_amount;
    let tax_exc = total.tax_exclusive_amount;
    let tax_inc = total.tax_inclusive_amount;
    let payable = total.payable_amount;

    let mut xml = String::from("  <cac:LegalMonetaryTotal>\n");
    xml.push_str(&format!(
        "    <cbc:LineExtensionAmount currencyID=\"TRY\">{line_ext}</cbc:LineExtensionAmount>\n\
         <cbc:TaxExclusiveAmount currencyID=\"TRY\">{tax_exc}</cbc:TaxExclusiveAmount>\n\
         <cbc:TaxInclusiveAmount currencyID=\"TRY\">{tax_inc}</cbc:TaxInclusiveAmount>\n"
    ));

    if let Some(allowance) = total.allowance_total_amount {
        xml.push_str(&format!(
            "    <cbc:AllowanceTotalAmount currencyID=\"TRY\">{allowance}</cbc:AllowanceTotalAmount>\n"
        ));
    }

    xml.push_str(&format!(
        "    <cbc:PayableAmount currencyID=\"TRY\">{payable}</cbc:PayableAmount>\n"
    ));
    xml.push_str("  </cac:LegalMonetaryTotal>\n");
    xml
}

// ---------------------------------------------------------------------------
// Public API: EFatura -> UBL-TR XML
// ---------------------------------------------------------------------------

/// Convert an EFatura domain object to a UBL-TR XML string.
///
/// Generates a complete UBL-TR Invoice document conforming to the
/// Turkish e-Fatura specification (UBL 2.1 based).
pub fn efatura_to_ubl_xml(fatura: &EFatura) -> Result<String, String> {
    if fatura.document_number.is_empty() {
        return Err("document_number must not be empty".to_string());
    }
    if fatura.uuid.is_empty() {
        return Err("uuid must not be empty".to_string());
    }
    if fatura.sender.vkn_tckn.is_empty() {
        return Err("sender VKN/TCKN must not be empty".to_string());
    }
    if fatura.receiver.vkn_tckn.is_empty() {
        return Err("receiver VKN/TCKN must not be empty".to_string());
    }

    let doc_number = xml_escape(&fatura.document_number);
    let uuid = xml_escape(&fatura.uuid);
    let issue_date = fatura.issue_date.format("%Y-%m-%d").to_string();
    let type_code = profile_to_type_code(&fatura.profile_id);
    let profile_id = fatura.profile_id.to_string();

    let mut xml = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <Invoice xmlns=\"{NS_INVOICE}\"\n\
         xmlns:cac=\"{NS_CAC}\"\n\
         xmlns:cbc=\"{NS_CBC}\">\n\
         <cbc:UBLVersionID>2.1</cbc:UBLVersionID>\n\
         <cbc:CustomizationID>urn:fdc:peppol.eu:2017:poacc:billing:3.0</cbc:CustomizationID>\n\
         <cbc:ProfileID>{profile_id}</cbc:ProfileID>\n\
         <cbc:ID>{doc_number}</cbc:ID>\n\
         <cbc:UUID>{uuid}</cbc:UUID>\n\
         <cbc:IssueDate>{issue_date}</cbc:IssueDate>\n\
         <cbc:InvoiceTypeCode>{type_code}</cbc:InvoiceTypeCode>\n"
    );

    // Sender
    xml.push_str(&format_party(&fatura.sender, "AccountingSupplierParty"));

    // Receiver
    xml.push_str(&format_party(&fatura.receiver, "AccountingCustomerParty"));

    // Tax totals
    if !fatura.tax_totals.is_empty() {
        xml.push_str("  <cac:TaxTotal>\n");
        for subtotal in &fatura.tax_totals {
            xml.push_str(&format_tax_total(subtotal));
        }
        xml.push_str("  </cac:TaxTotal>\n");
    }

    // Monetary total
    xml.push_str(&format_monetary_total(&fatura.legal_monetary_total));

    // Invoice lines
    for (i, line) in fatura.lines.iter().enumerate() {
        xml.push_str(&format_line(line, i));
    }

    xml.push_str("</Invoice>\n");
    Ok(xml)
}

// ---------------------------------------------------------------------------
// Public API: UBL-TR XML -> UblPartialInvoice
// ---------------------------------------------------------------------------

/// Extract a simplified invoice representation from a UBL-TR XML string.
///
/// This performs a basic text-based parse (no full XML parser) and extracts
/// the key header fields. It is intentionally lenient: missing optional
/// fields are returned as empty strings / zero.
pub fn ubl_xml_to_efatura_partial(xml: &str) -> Result<UblPartialInvoice, String> {
    if xml.trim().is_empty() {
        return Err("XML string is empty".to_string());
    }

    // Check for Invoice root element
    if !xml.contains("<Invoice") {
        return Err("XML does not contain an Invoice root element".to_string());
    }

    let document_number = extract_tag_value(xml, "cbc:ID").unwrap_or_default();
    let uuid = extract_tag_value(xml, "cbc:UUID").unwrap_or_default();
    let issue_date = extract_tag_value(xml, "cbc:IssueDate").unwrap_or_default();

    // Find sender/receiver by looking within their parent sections
    let sender_vkn =
        extract_nested_value(xml, "AccountingSupplierParty", "cbc:VKN").unwrap_or_default();
    let sender_name =
        extract_nested_value(xml, "AccountingSupplierParty", "cbc:Name").unwrap_or_default();
    let receiver_vkn =
        extract_nested_value(xml, "AccountingCustomerParty", "cbc:VKN").unwrap_or_default();
    let receiver_name =
        extract_nested_value(xml, "AccountingCustomerParty", "cbc:Name").unwrap_or_default();

    // Count InvoiceLine elements
    let line_count = xml.matches("<cac:InvoiceLine").count();

    // Extract tax inclusive amount
    let tax_inclusive_amount = extract_tag_value(xml, "cbc:TaxInclusiveAmount")
        .and_then(|s| s.parse::<Decimal>().ok())
        .unwrap_or(Decimal::ZERO);

    Ok(UblPartialInvoice {
        document_number,
        uuid,
        issue_date,
        sender_vkn,
        sender_name,
        receiver_vkn,
        receiver_name,
        line_count,
        tax_inclusive_amount,
    })
}

// ---------------------------------------------------------------------------
// Internal parsing helpers
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
        // Find the closing '>' of the opening tag
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

/// Extract the text content of a tag that appears within a specific parent section.
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
        AddressInfo, EFaturaProfile, EFaturaStatus, MonetaryTotal, PartyInfo,
    };
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
                unit_price: Decimal::new(1000, 2),   // 10.00
                line_amount: Decimal::new(10000, 2), // 100.00
                tax_rate: Decimal::new(18, 1),       // 1.8 = 18%
                tax_amount: Decimal::new(1800, 2),   // 18.00
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
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn test_generate_ubl_xml_basic() {
        let fatura = sample_fatura();
        let xml = efatura_to_ubl_xml(&fatura).unwrap();

        // Verify structure
        assert!(xml.contains("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
        assert!(xml.contains("<Invoice"));
        assert!(xml.contains("xmlns=\"urn:oasis:names:specification:ubl:schema:xsd:Invoice-2\""));
        assert!(xml.contains("<cbc:UBLVersionID>2.1</cbc:UBLVersionID>"));
        assert!(xml.contains("<cbc:ProfileID>TEMELFATURA</cbc:ProfileID>"));
        assert!(xml.contains("<cbc:ID>ABC2024000000001</cbc:ID>"));
        assert!(xml.contains("<cbc:UUID>3f2504e0-4f89-11d3-9a0c-0305e82c3301</cbc:UUID>"));
        assert!(xml.contains("<cbc:IssueDate>2024-06-15</cbc:IssueDate>"));
        assert!(xml.contains("<cbc:InvoiceTypeCode>381</cbc:InvoiceTypeCode>"));

        // Sender
        assert!(xml.contains("<cbc:VKN>1234567890</cbc:VKN>"));
        assert!(xml.contains("<cbc:Name>Acme Corp</cbc:Name>"));

        // Receiver
        assert!(xml.contains("<cbc:VKN>9876543210</cbc:VKN>"));
        assert!(xml.contains("<cbc:Name>Buyer Ltd</cbc:Name>"));

        // Line items
        assert!(xml.contains("<cac:InvoiceLine>"));
        assert!(xml.contains("<cbc:Name>Widget A</cbc:Name>"));

        // Monetary totals
        assert!(xml.contains("<cbc:PayableAmount currencyID=\"TRY\">"));
        assert!(xml.contains("</Invoice>"));
    }

    #[test]
    fn test_efatura_to_ubl_xml_empty_fields() {
        let mut fatura = sample_fatura();
        fatura.document_number = String::new();
        let result = efatura_to_ubl_xml(&fatura);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("document_number"));

        fatura.document_number = "ABC2024000000001".to_string();
        fatura.uuid = String::new();
        let result = efatura_to_ubl_xml(&fatura);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("uuid"));
    }

    #[test]
    fn test_xml_escape() {
        assert_eq!(
            xml_escape("A & B < C > D \"E\" 'F'"),
            "A &amp; B &lt; C &gt; D &quot;E&quot; &apos;F&apos;"
        );
        assert_eq!(xml_escape("Hello World"), "Hello World");
    }

    #[test]
    fn test_profile_to_type_code() {
        assert_eq!(profile_to_type_code(&EFaturaProfile::TemelFatura), "381");
        assert_eq!(profile_to_type_code(&EFaturaProfile::Ihracat), "382");
        assert_eq!(profile_to_type_code(&EFaturaProfile::YolcuBeleni), "383");
        assert_eq!(
            profile_to_type_code(&EFaturaProfile::OzelMatbuFatura),
            "381"
        );
    }

    #[test]
    fn test_parse_ubl_xml() {
        let fatura = sample_fatura();
        let xml = efatura_to_ubl_xml(&fatura).unwrap();
        let partial = ubl_xml_to_efatura_partial(&xml).unwrap();

        assert_eq!(partial.document_number, "ABC2024000000001");
        assert_eq!(partial.uuid, "3f2504e0-4f89-11d3-9a0c-0305e82c3301");
        assert_eq!(partial.issue_date, "2024-06-15");
        assert_eq!(partial.sender_vkn, "1234567890");
        assert_eq!(partial.sender_name, "Acme Corp");
        assert_eq!(partial.receiver_vkn, "9876543210");
        assert_eq!(partial.receiver_name, "Buyer Ltd");
        assert_eq!(partial.line_count, 1);
        assert_eq!(partial.tax_inclusive_amount, Decimal::new(11800, 2));
    }

    #[test]
    fn test_parse_ubl_xml_empty() {
        let result = ubl_xml_to_efatura_partial("");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("empty"));

        let result = ubl_xml_to_efatura_partial("<NotInvoice/>");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invoice"));
    }

    #[test]
    fn test_parse_ubl_xml_multiple_lines() {
        let mut fatura = sample_fatura();
        fatura.lines = vec![
            EFaturaLine {
                id: "1".to_string(),
                product_name: "Item 1".to_string(),
                quantity: Decimal::new(5, 0),
                unit: "C62".to_string(),
                unit_price: Decimal::new(2000, 2),
                line_amount: Decimal::new(10000, 2),
                tax_rate: Decimal::new(18, 1),
                tax_amount: Decimal::new(900, 2),
            },
            EFaturaLine {
                id: "2".to_string(),
                product_name: "Item 2".to_string(),
                quantity: Decimal::new(3, 0),
                unit: "C62".to_string(),
                unit_price: Decimal::new(5000, 2),
                line_amount: Decimal::new(15000, 2),
                tax_rate: Decimal::new(18, 1),
                tax_amount: Decimal::new(2700, 2),
            },
        ];

        let xml = efatura_to_ubl_xml(&fatura).unwrap();
        let partial = ubl_xml_to_efatura_partial(&xml).unwrap();
        assert_eq!(partial.line_count, 2);
    }

    #[test]
    fn test_extract_tag_value() {
        let xml = "<cbc:ID>ABC123</cbc:ID><cbc:Name>Test</cbc:Name>";
        assert_eq!(extract_tag_value(xml, "cbc:ID"), Some("ABC123".to_string()));
        assert_eq!(extract_tag_value(xml, "cbc:Name"), Some("Test".to_string()));
        assert_eq!(extract_tag_value(xml, "cbc:Missing"), None);
    }

    #[test]
    fn test_extract_nested_value() {
        let xml = "<cac:AccountingSupplierParty><cbc:VKN>111</cbc:VKN><cbc:Name>Sender</cbc:Name></cac:AccountingSupplierParty><cac:AccountingCustomerParty><cbc:VKN>222</cbc:VKN><cbc:Name>Receiver</cbc:Name></cac:AccountingCustomerParty>";
        assert_eq!(
            extract_nested_value(xml, "AccountingSupplierParty", "cbc:VKN"),
            Some("111".to_string())
        );
        assert_eq!(
            extract_nested_value(xml, "AccountingCustomerParty", "cbc:VKN"),
            Some("222".to_string())
        );
    }
}
