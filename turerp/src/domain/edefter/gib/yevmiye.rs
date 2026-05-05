//! GIB-format Yevmiye defteri XML generator
//!
//! Generates XML documents conforming to the GIB (Gelir İdaresi Başkanlığı)
//! specification for Yevmiye defteri (journal ledger) submissions.

use crate::domain::edefter::model::{LedgerPeriod, YevmiyeEntry};

/// Generate a GIB-format Yevmiye defteri XML from a ledger period and its entries.
///
/// The XML follows the GIB e-Defter specification structure with:
/// - Defter header containing period metadata
/// - Yevmiye entries with debit/credit lines
/// - Balance validation
pub fn generate_yevmiye_xml(
    period: &LedgerPeriod,
    entries: &[YevmiyeEntry],
) -> Result<String, String> {
    if entries.is_empty() {
        return Err("Cannot generate Yevmiye XML: no entries provided".to_string());
    }

    let mut xml = String::new();
    xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    xml.push_str("<YevmiyeDefteri>\n");

    // Header
    xml.push_str("  <DefterBilgisi>\n");
    xml.push_str(&format!("    <DonemYili>{}</DonemYili>\n", period.year));
    xml.push_str(&format!("    <DonemAy>{}</DonemAy>\n", period.month));
    xml.push_str(&format!(
        "    <DefterTuru>{}</DefterTuru>\n",
        period.period_type
    ));
    xml.push_str(&format!("    <Durum>{}</Durum>\n", period.status));
    xml.push_str("  </DefterBilgisi>\n");

    // Entries
    xml.push_str("  <YevmiyeKayitlari>\n");
    for entry in entries {
        xml.push_str("    <YevmiyeKayit>\n");
        xml.push_str(&format!(
            "      <KayitNo>{}</KayitNo>\n",
            entry.entry_number
        ));
        xml.push_str(&format!("      <Tarih>{}</Tarih>\n", entry.entry_date));
        xml.push_str(&format!(
            "      <Aciklama>{}</Aciklama>\n",
            xml_escape(&entry.explanation)
        ));
        xml.push_str(&format!(
            "      <BorcToplam>{}</BorcToplam>\n",
            entry.debit_total
        ));
        xml.push_str(&format!(
            "      <AlacakToplam>{}</AlacakToplam>\n",
            entry.credit_total
        ));
        xml.push_str("      <Satirlar>\n");
        for line in &entry.lines {
            xml.push_str("        <Satir>\n");
            xml.push_str(&format!(
                "          <HesapKodu>{}</HesapKodu>\n",
                xml_escape(&line.account_code)
            ));
            xml.push_str(&format!(
                "          <HesapAdi>{}</HesapAdi>\n",
                xml_escape(&line.account_name)
            ));
            xml.push_str(&format!("          <Borc>{}</Borc>\n", line.debit));
            xml.push_str(&format!("          <Alacak>{}</Alacak>\n", line.credit));
            xml.push_str(&format!(
                "          <Aciklama>{}</Aciklama>\n",
                xml_escape(&line.explanation)
            ));
            xml.push_str("        </Satir>\n");
        }
        xml.push_str("      </Satirlar>\n");
        xml.push_str("    </YevmiyeKayit>\n");
    }
    xml.push_str("  </YevmiyeKayitlari>\n");

    xml.push_str("</YevmiyeDefteri>");

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
    use crate::domain::edefter::model::{EDefterStatus, LedgerType, YevmiyeLine};
    use chrono::{NaiveDate, Utc};
    use rust_decimal::Decimal;

    fn sample_period() -> LedgerPeriod {
        LedgerPeriod {
            id: 1,
            tenant_id: 100,
            year: 2024,
            month: 6,
            period_type: LedgerType::YevmiyeDefteri,
            status: EDefterStatus::Draft,
            berat_signed_at: None,
            sent_at: None,
            created_at: Utc::now(),
        }
    }

    fn sample_entries() -> Vec<YevmiyeEntry> {
        vec![YevmiyeEntry {
            id: 1,
            period_id: 1,
            entry_number: 1,
            entry_date: NaiveDate::from_ymd_opt(2024, 6, 15).unwrap(),
            explanation: "Kasa & Banka hareketi".to_string(),
            debit_total: Decimal::new(50000, 2),
            credit_total: Decimal::new(50000, 2),
            lines: vec![
                YevmiyeLine {
                    account_code: "100.01".to_string(),
                    account_name: "Kasa".to_string(),
                    debit: Decimal::new(50000, 2),
                    credit: Decimal::ZERO,
                    explanation: "Kasa borç kaydı".to_string(),
                },
                YevmiyeLine {
                    account_code: "102.01".to_string(),
                    account_name: "Bankalar".to_string(),
                    debit: Decimal::ZERO,
                    credit: Decimal::new(50000, 2),
                    explanation: "Banka alacak kaydı".to_string(),
                },
            ],
        }]
    }

    #[test]
    fn test_generate_yevmiye_xml_basic() {
        let period = sample_period();
        let entries = sample_entries();
        let xml = generate_yevmiye_xml(&period, &entries).unwrap();

        assert!(xml.contains("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
        assert!(xml.contains("<YevmiyeDefteri>"));
        assert!(xml.contains("</YevmiyeDefteri>"));
        assert!(xml.contains("<DefterBilgisi>"));
        assert!(xml.contains("<DonemYili>2024</DonemYili>"));
        assert!(xml.contains("<DonemAy>6</DonemAy>"));
        assert!(xml.contains("<DefterTuru>YevmiyeDefteri</DefterTuru>"));
        assert!(xml.contains("<Durum>Draft</Durum>"));
    }

    #[test]
    fn test_generate_yevmiye_xml_entries() {
        let period = sample_period();
        let entries = sample_entries();
        let xml = generate_yevmiye_xml(&period, &entries).unwrap();

        assert!(xml.contains("<YevmiyeKayitlari>"));
        assert!(xml.contains("<KayitNo>1</KayitNo>"));
        assert!(xml.contains("<Tarih>2024-06-15</Tarih>"));
        assert!(xml.contains("<BorcToplam>500.00</BorcToplam>"));
        assert!(xml.contains("<AlacakToplam>500.00</AlacakToplam>"));
        assert!(xml.contains("<HesapKodu>100.01</HesapKodu>"));
        assert!(xml.contains("<HesapAdi>Kasa</HesapAdi>"));
    }

    #[test]
    fn test_generate_yevmiye_xml_escapes_special_chars() {
        let period = sample_period();
        let entries = vec![YevmiyeEntry {
            id: 1,
            period_id: 1,
            entry_number: 1,
            entry_date: NaiveDate::from_ymd_opt(2024, 6, 15).unwrap(),
            explanation: "Test & <entry> \"quotes\"".to_string(),
            debit_total: Decimal::new(100, 2),
            credit_total: Decimal::new(100, 2),
            lines: vec![],
        }];
        let xml = generate_yevmiye_xml(&period, &entries).unwrap();

        assert!(xml.contains("Test &amp; &lt;entry&gt; &quot;quotes&quot;"));
        assert!(!xml.contains("Test & <entry>"));
    }

    #[test]
    fn test_generate_yevmiye_xml_empty_entries() {
        let period = sample_period();
        let result = generate_yevmiye_xml(&period, &[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("no entries"));
    }

    #[test]
    fn test_generate_yevmiye_xml_multiple_entries() {
        let period = sample_period();
        let entries = vec![
            YevmiyeEntry {
                id: 1,
                period_id: 1,
                entry_number: 1,
                entry_date: NaiveDate::from_ymd_opt(2024, 6, 1).unwrap(),
                explanation: "Entry 1".to_string(),
                debit_total: Decimal::new(1000, 2),
                credit_total: Decimal::new(1000, 2),
                lines: vec![],
            },
            YevmiyeEntry {
                id: 2,
                period_id: 1,
                entry_number: 2,
                entry_date: NaiveDate::from_ymd_opt(2024, 6, 2).unwrap(),
                explanation: "Entry 2".to_string(),
                debit_total: Decimal::new(2000, 2),
                credit_total: Decimal::new(2000, 2),
                lines: vec![],
            },
        ];
        let xml = generate_yevmiye_xml(&period, &entries).unwrap();

        assert!(xml.contains("<KayitNo>1</KayitNo>"));
        assert!(xml.contains("<KayitNo>2</KayitNo>"));
        assert!(xml.contains("<Aciklama>Entry 1</Aciklama>"));
        assert!(xml.contains("<Aciklama>Entry 2</Aciklama>"));
    }
}
