//! GIB-format Büyük Defter XML generator
//!
//! Generates XML documents conforming to the GIB (Gelir İdaresi Başkanlığı)
//! specification for Büyük defter (general ledger) submissions.
//! Groups Yevmiye entries by account code for the Büyük defter format.

use std::collections::BTreeMap;

use crate::domain::edefter::model::{LedgerPeriod, YevmiyeEntry};
use rust_decimal::Decimal;

/// An account's aggregated entries for Büyük defter output.
#[derive(Debug, Clone)]
struct AccountEntry {
    account_code: String,
    account_name: String,
    lines: Vec<BuyukDefterLine>,
    total_debit: Decimal,
    total_credit: Decimal,
}

/// A single line within a Büyük defter account group.
#[derive(Debug, Clone)]
struct BuyukDefterLine {
    entry_number: i64,
    entry_date: String,
    explanation: String,
    debit: Decimal,
    credit: Decimal,
}

/// Generate a GIB-format Büyük defter XML from a ledger period and its Yevmiye entries.
///
/// Unlike the Yevmiye defteri (chronological), the Büyük defter groups entries
/// by account code, showing all movements per account.
pub fn generate_buyuk_defter_xml(
    period: &LedgerPeriod,
    entries: &[YevmiyeEntry],
) -> Result<String, String> {
    if entries.is_empty() {
        return Err("Cannot generate Büyük Defter XML: no entries provided".to_string());
    }

    let accounts = group_by_account(entries);

    let mut xml = String::new();
    xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    xml.push_str("<BuyukDefter>\n");

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

    // Account groups
    xml.push_str("  <HesapKayitlari>\n");
    for account in &accounts {
        xml.push_str("    <HesapKayit>\n");
        xml.push_str(&format!(
            "      <HesapKodu>{}</HesapKodu>\n",
            xml_escape(&account.account_code)
        ));
        xml.push_str(&format!(
            "      <HesapAdi>{}</HesapAdi>\n",
            xml_escape(&account.account_name)
        ));
        xml.push_str(&format!(
            "      <ToplamBorc>{}</ToplamBorc>\n",
            account.total_debit
        ));
        xml.push_str(&format!(
            "      <ToplamAlacak>{}</ToplamAlacak>\n",
            account.total_credit
        ));
        xml.push_str("      <Satirlar>\n");
        for line in &account.lines {
            xml.push_str("        <Satir>\n");
            xml.push_str(&format!(
                "          <KayitNo>{}</KayitNo>\n",
                line.entry_number
            ));
            xml.push_str(&format!("          <Tarih>{}</Tarih>\n", line.entry_date));
            xml.push_str(&format!(
                "          <Aciklama>{}</Aciklama>\n",
                xml_escape(&line.explanation)
            ));
            xml.push_str(&format!("          <Borc>{}</Borc>\n", line.debit));
            xml.push_str(&format!("          <Alacak>{}</Alacak>\n", line.credit));
            xml.push_str("        </Satir>\n");
        }
        xml.push_str("      </Satirlar>\n");
        xml.push_str("    </HesapKayit>\n");
    }
    xml.push_str("  </HesapKayitlari>\n");

    xml.push_str("</BuyukDefter>");

    Ok(xml)
}

/// Group Yevmiye entries by account code, aggregating lines per account.
fn group_by_account(entries: &[YevmiyeEntry]) -> Vec<AccountEntry> {
    let mut map: BTreeMap<String, AccountEntry> = BTreeMap::new();

    for entry in entries {
        for line in &entry.lines {
            let account = map
                .entry(line.account_code.clone())
                .or_insert_with(|| AccountEntry {
                    account_code: line.account_code.clone(),
                    account_name: line.account_name.clone(),
                    lines: Vec::new(),
                    total_debit: Decimal::ZERO,
                    total_credit: Decimal::ZERO,
                });

            account.lines.push(BuyukDefterLine {
                entry_number: entry.entry_number,
                entry_date: entry.entry_date.to_string(),
                explanation: line.explanation.clone(),
                debit: line.debit,
                credit: line.credit,
            });

            account.total_debit += line.debit;
            account.total_credit += line.credit;
        }
    }

    map.into_values().collect()
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
            period_type: LedgerType::BuyukDefter,
            status: EDefterStatus::Draft,
            berat_signed_at: None,
            sent_at: None,
            created_at: Utc::now(),
        }
    }

    fn sample_entries() -> Vec<YevmiyeEntry> {
        vec![
            YevmiyeEntry {
                id: 1,
                period_id: 1,
                entry_number: 1,
                entry_date: NaiveDate::from_ymd_opt(2024, 6, 1).unwrap(),
                explanation: "Kasa hareketi".to_string(),
                debit_total: Decimal::new(50000, 2),
                credit_total: Decimal::new(50000, 2),
                lines: vec![
                    YevmiyeLine {
                        account_code: "100.01".to_string(),
                        account_name: "Kasa".to_string(),
                        debit: Decimal::new(50000, 2),
                        credit: Decimal::ZERO,
                        explanation: "Kasa borç".to_string(),
                    },
                    YevmiyeLine {
                        account_code: "102.01".to_string(),
                        account_name: "Bankalar".to_string(),
                        debit: Decimal::ZERO,
                        credit: Decimal::new(50000, 2),
                        explanation: "Banka alacak".to_string(),
                    },
                ],
            },
            YevmiyeEntry {
                id: 2,
                period_id: 1,
                entry_number: 2,
                entry_date: NaiveDate::from_ymd_opt(2024, 6, 5).unwrap(),
                explanation: "Banka hareketi".to_string(),
                debit_total: Decimal::new(30000, 2),
                credit_total: Decimal::new(30000, 2),
                lines: vec![
                    YevmiyeLine {
                        account_code: "102.01".to_string(),
                        account_name: "Bankalar".to_string(),
                        debit: Decimal::new(30000, 2),
                        credit: Decimal::ZERO,
                        explanation: "Banka borç".to_string(),
                    },
                    YevmiyeLine {
                        account_code: "100.01".to_string(),
                        account_name: "Kasa".to_string(),
                        debit: Decimal::ZERO,
                        credit: Decimal::new(30000, 2),
                        explanation: "Kasa alacak".to_string(),
                    },
                ],
            },
        ]
    }

    #[test]
    fn test_generate_buyuk_defter_xml_basic() {
        let period = sample_period();
        let entries = sample_entries();
        let xml = generate_buyuk_defter_xml(&period, &entries).unwrap();

        assert!(xml.contains("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
        assert!(xml.contains("<BuyukDefter>"));
        assert!(xml.contains("</BuyukDefter>"));
        assert!(xml.contains("<DefterBilgisi>"));
        assert!(xml.contains("<DonemYili>2024</DonemYili>"));
        assert!(xml.contains("<DefterTuru>BuyukDefter</DefterTuru>"));
    }

    #[test]
    fn test_generate_buyuk_defter_groups_by_account() {
        let period = sample_period();
        let entries = sample_entries();
        let xml = generate_buyuk_defter_xml(&period, &entries).unwrap();

        // Should have account groups
        assert!(xml.contains("<HesapKayitlari>"));
        assert!(xml.contains("<HesapKodu>100.01</HesapKodu>"));
        assert!(xml.contains("<HesapKodu>102.01</HesapKodu>"));

        // Account 100.01 (Kasa) has two entries aggregated
        // From entry 1: debit 500.00
        // From entry 2: credit 300.00
        assert!(xml.contains("<HesapAdi>Kasa</HesapAdi>"));
        assert!(xml.contains("<HesapAdi>Bankalar</HesapAdi>"));
    }

    #[test]
    fn test_generate_buyuk_defter_totals() {
        let period = sample_period();
        let entries = sample_entries();
        let xml = generate_buyuk_defter_xml(&period, &entries).unwrap();

        // Kasa account: total debit 500.00, total credit 300.00
        // Bankalar account: total debit 300.00, total credit 500.00
        assert!(xml.contains("<ToplamBorc>500.00</ToplamBorc>"));
        assert!(xml.contains("<ToplamAlacak>500.00</ToplamAlacak>"));
    }

    #[test]
    fn test_generate_buyuk_defter_xml_empty_entries() {
        let period = sample_period();
        let result = generate_buyuk_defter_xml(&period, &[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("no entries"));
    }

    #[test]
    fn test_generate_buyuk_defter_xml_escapes() {
        let period = sample_period();
        let entries = vec![YevmiyeEntry {
            id: 1,
            period_id: 1,
            entry_number: 1,
            entry_date: NaiveDate::from_ymd_opt(2024, 6, 1).unwrap(),
            explanation: "Test".to_string(),
            debit_total: Decimal::new(100, 2),
            credit_total: Decimal::new(100, 2),
            lines: vec![YevmiyeLine {
                account_code: "100.01".to_string(),
                account_name: "Kasa & Bankalar".to_string(),
                debit: Decimal::new(100, 2),
                credit: Decimal::ZERO,
                explanation: "Test <entry>".to_string(),
            }],
        }];
        let xml = generate_buyuk_defter_xml(&period, &entries).unwrap();

        assert!(xml.contains("Kasa &amp; Bankalar"));
        assert!(xml.contains("Test &lt;entry&gt;"));
    }

    #[test]
    fn test_group_by_account_ordering() {
        let period = sample_period();
        let entries = sample_entries();
        let xml = generate_buyuk_defter_xml(&period, &entries).unwrap();

        // BTreeMap ensures 100.01 comes before 102.01
        let pos_100 = xml.find("<HesapKodu>100.01</HesapKodu>").unwrap();
        let pos_102 = xml.find("<HesapKodu>102.01</HesapKodu>").unwrap();
        assert!(
            pos_100 < pos_102,
            "100.01 should appear before 102.01 in XML output"
        );
    }
}
