//! UBL-TR XML report generation for Turkish e-Defter compliance

use super::{ReportError, ReportRequest};

pub fn generate_edefter_xml(request: &ReportRequest) -> Result<Vec<u8>, ReportError> {
    let params = &request.parameters;
    let period = params
        .get("period")
        .and_then(|v| v.as_str())
        .unwrap_or("2026-01");
    let entries = params
        .get("entries")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let mut xml = String::new();
    xml.push('\u{feff}');
    xml.push_str(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
    xml.push('\n');
    xml.push_str(r#"<GenericAccountingPacket xmlns="urn:gi:eFatura:ubl:GenericAccountingPacket">"#);
    xml.push('\n');
    xml.push_str("  <PacketInfo>\n");
    xml.push_str("    <PacketVersId>1</PacketVersId>\n");
    xml.push_str("    <PacketType>GENELMUHASEBE</PacketType>\n");
    xml.push_str(&format!("    <Period>{}</Period>\n", period));
    xml.push_str(&format!("    <TenantId>{}</TenantId>\n", request.tenant_id));
    xml.push_str(
        &"    <GenerationDate>{}</GenerationDate>\n"
            .replace("{}", &chrono::Utc::now().to_rfc3339()),
    );
    xml.push_str("  </PacketInfo>\n");

    if !entries.is_empty() {
        xml.push_str("  <Entries>\n");
        for entry in &entries {
            let date = entry.get("date").and_then(|v| v.as_str()).unwrap_or("");
            let account = entry
                .get("account_code")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let desc = entry
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let debit = entry.get("debit").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let credit = entry.get("credit").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let reference = entry
                .get("reference")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            xml.push_str("    <Entry>\n");
            xml.push_str(&format!("      <Date>{}</Date>\n", escape_xml(date)));
            xml.push_str(&format!(
                "      <AccountCode>{}</AccountCode>\n",
                escape_xml(account)
            ));
            xml.push_str(&format!(
                "      <Description>{}</Description>\n",
                escape_xml(desc)
            ));
            xml.push_str(&format!("      <Debit>{:.2}</Debit>\n", debit));
            xml.push_str(&format!("      <Credit>{:.2}</Credit>\n", credit));
            xml.push_str(&format!(
                "      <Reference>{}</Reference>\n",
                escape_xml(reference)
            ));
            xml.push_str("    </Entry>\n");
        }
        xml.push_str("  </Entries>\n");
    }

    xml.push_str("</GenericAccountingPacket>\n");

    Ok(xml.into_bytes())
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
