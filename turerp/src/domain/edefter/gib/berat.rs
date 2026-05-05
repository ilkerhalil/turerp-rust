//! GIB-format Berat (certificate) XML generator
//!
//! Generates XML documents conforming to the GIB specification for Berat
//! signing structures used to certify e-Defter submissions.

use crate::domain::edefter::model::{BeratInfo, LedgerPeriod};

/// Generate a GIB-format Berat XML from a ledger period and its berat info.
///
/// The Berat is a digital certificate that signs and certifies the ledger
/// submission, containing the signer information, digest, and signature.
pub fn generate_berat_xml(period: &LedgerPeriod, berat: &BeratInfo) -> Result<String, String> {
    let mut xml = String::new();
    xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    xml.push_str("<Berat>\n");

    // Period reference
    xml.push_str("  <DefterBilgisi>\n");
    xml.push_str(&format!("    <DonemYili>{}</DonemYili>\n", period.year));
    xml.push_str(&format!("    <DonemAy>{}</DonemAy>\n", period.month));
    xml.push_str(&format!(
        "    <DefterTuru>{}</DefterTuru>\n",
        period.period_type
    ));
    xml.push_str("  </DefterBilgisi>\n");

    // Berat details
    xml.push_str("  <BeratBilgisi>\n");
    xml.push_str(&format!(
        "    <SeriNo>{}</SeriNo>\n",
        xml_escape(&berat.serial_number)
    ));
    xml.push_str(&format!(
        "    <ImzaZamani>{}</ImzaZamani>\n",
        berat.sign_time.to_rfc3339()
    ));
    xml.push_str(&format!(
        "    <Imzalayan>{}</Imzalayan>\n",
        xml_escape(&berat.signer)
    ));
    xml.push_str("  </BeratBilgisi>\n");

    // Signature block
    xml.push_str("  <Imza>\n");
    xml.push_str(&format!(
        "    <OzetDegeri>{}</OzetDegeri>\n",
        xml_escape(&berat.digest_value)
    ));
    xml.push_str(&format!(
        "    <ImzaDegeri>{}</ImzaDegeri>\n",
        xml_escape(&berat.signature_value)
    ));
    xml.push_str("  </Imza>\n");

    xml.push_str("</Berat>");

    Ok(xml)
}

/// Escape special XML characters in text content.
fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::edefter::model::{EDefterStatus, LedgerType};
    use chrono::Utc;

    fn sample_period() -> LedgerPeriod {
        LedgerPeriod {
            id: 1,
            tenant_id: 100,
            year: 2024,
            month: 6,
            period_type: LedgerType::YevmiyeDefteri,
            status: EDefterStatus::Signed,
            berat_signed_at: None,
            sent_at: None,
            created_at: Utc::now(),
        }
    }

    fn sample_berat(period_id: i64) -> BeratInfo {
        BeratInfo {
            period_id,
            serial_number: "BERAT-2024-001".to_string(),
            sign_time: Utc::now(),
            signer: "Test Signer & Associates".to_string(),
            digest_value: "sha256-abc123def456".to_string(),
            signature_value: "RSA-sig789ghi012jkl345".to_string(),
        }
    }

    #[test]
    fn test_generate_berat_xml_basic() {
        let period = sample_period();
        let berat = sample_berat(period.id);
        let xml = generate_berat_xml(&period, &berat).unwrap();

        assert!(xml.contains("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
        assert!(xml.contains("<Berat>"));
        assert!(xml.contains("</Berat>"));
    }

    #[test]
    fn test_generate_berat_xml_period_info() {
        let period = sample_period();
        let berat = sample_berat(period.id);
        let xml = generate_berat_xml(&period, &berat).unwrap();

        assert!(xml.contains("<DefterBilgisi>"));
        assert!(xml.contains("<DonemYili>2024</DonemYili>"));
        assert!(xml.contains("<DonemAy>6</DonemAy>"));
        assert!(xml.contains("<DefterTuru>YevmiyeDefteri</DefterTuru>"));
    }

    #[test]
    fn test_generate_berat_xml_berat_info() {
        let period = sample_period();
        let berat = sample_berat(period.id);
        let xml = generate_berat_xml(&period, &berat).unwrap();

        assert!(xml.contains("<BeratBilgisi>"));
        assert!(xml.contains("<SeriNo>BERAT-2024-001</SeriNo>"));
        assert!(xml.contains("<ImzaZamani>"));
        assert!(xml.contains("<Imzalayan>Test Signer &amp; Associates</Imzalayan>"));
    }

    #[test]
    fn test_generate_berat_xml_signature() {
        let period = sample_period();
        let berat = sample_berat(period.id);
        let xml = generate_berat_xml(&period, &berat).unwrap();

        assert!(xml.contains("<Imza>"));
        assert!(xml.contains("<OzetDegeri>sha256-abc123def456</OzetDegeri>"));
        assert!(xml.contains("<ImzaDegeri>RSA-sig789ghi012jkl345</ImzaDegeri>"));
    }

    #[test]
    fn test_generate_berat_xml_escapes_special_chars() {
        let period = sample_period();
        let berat = BeratInfo {
            period_id: period.id,
            serial_number: "SN<1>&2\"3'".to_string(),
            sign_time: Utc::now(),
            signer: "A & B < C > D".to_string(),
            digest_value: "digest".to_string(),
            signature_value: "signature".to_string(),
        };
        let xml = generate_berat_xml(&period, &berat).unwrap();

        assert!(xml.contains("SN&lt;1&gt;&amp;2&quot;3&apos;"));
        assert!(xml.contains("A &amp; B &lt; C &gt; D"));
    }

    #[test]
    fn test_generate_berat_xml_buyuk_defter() {
        let mut period = sample_period();
        period.period_type = LedgerType::BuyukDefter;
        let berat = sample_berat(period.id);
        let xml = generate_berat_xml(&period, &berat).unwrap();

        assert!(xml.contains("<DefterTuru>BuyukDefter</DefterTuru>"));
    }
}
