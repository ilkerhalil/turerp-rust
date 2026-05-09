//! Bank statement parsers for Turkish banks
//!
//! Supports MT940, CAMT.053 (ISO 20022 XML), and bank-specific XML formats.

use chrono::NaiveDate;
use regex::Regex;
use rust_decimal::Decimal;
use std::sync::LazyLock;

use crate::domain::bank::model::{BankCode, ParsedBankTransaction};

// MT940 :61: tag pattern
// Format: :61:2301010101C1000,00NTRFNONREF//REF123
// Date (6) + entry_date (4) + D/C + amount + N + transaction_type + reference
static MT940_61_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r":61:(\d{6})(?:\d{4})?([CD])([\d,]+)N([A-Z]{3})([^/]*)(?://(.*))?").unwrap()
});

// MT940 :86: tag pattern (description)
#[allow(dead_code)]
static MT940_86_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r":86:(.+?)(?:\n|:[0-9A-Z]{2,3}:|$)").unwrap());

// CAMT.053 basic XML patterns
static CAMT_DATE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?s)<Dt>(\d{4}-\d{2}-\d{2})</Dt>").unwrap());
static CAMT_TXDTL_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?s)<NtryDtls>.*?</NtryDtls>").unwrap());
static CAMT_AMOUNT_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?s)<Amt Ccy=\"([A-Z]{3})\">([\d.]+)</Amt>"#).unwrap());
static CAMT_CREDIT_DEBIT_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?s)<CdtDbtInd>([A-Za-z]+)</CdtDbtInd>").unwrap());
static CAMT_RMT_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?s)<RmtInf>.*?<Ustrd>(.+?)</Ustrd>.*?</RmtInf>").unwrap());
static CAMT_REF_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?s)<Refs>.*?<EndToEndId>(.+?)</EndToEndId>.*?</Refs>").unwrap());

/// Parse MT940 format statement data
pub fn parse_mt940(data: &str) -> Vec<ParsedBankTransaction> {
    let mut transactions = Vec::new();
    let mut current_description = String::new();
    let mut current_61_data: Option<(&str, &str, &str, &str, &str)> = None;

    for line in data.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        if line.starts_with(":61:") {
            // Save previous transaction if exists
            if let Some((date, cd, amount_str, _tx_type, ref_no)) = current_61_data.take() {
                let tx = build_mt940_tx(date, cd, amount_str, &current_description, ref_no);
                transactions.push(tx);
                current_description.clear();
            }

            if let Some(caps) = MT940_61_RE.captures(line) {
                let date = caps.get(1).map(|m| m.as_str()).unwrap_or("000101");
                let cd = caps.get(2).map(|m| m.as_str()).unwrap_or("C");
                let amount = caps.get(3).map(|m| m.as_str()).unwrap_or("0");
                let tx_type = caps.get(4).map(|m| m.as_str()).unwrap_or("TRF");
                let ref_no = caps
                    .get(6)
                    .or_else(|| caps.get(5))
                    .map(|m| m.as_str())
                    .unwrap_or("");
                current_61_data = Some((date, cd, amount, tx_type, ref_no));
            }
        } else if line.starts_with(":86:") {
            let desc = line.trim_start_matches(":86:").trim();
            current_description.push_str(desc);
            current_description.push(' ');
        }
    }

    // Save last transaction
    if let Some((date, cd, amount_str, _tx_type, ref_no)) = current_61_data.take() {
        let tx = build_mt940_tx(date, cd, amount_str, &current_description, ref_no);
        transactions.push(tx);
    }

    transactions
        .into_iter()
        .filter(|t| t.amount != Decimal::ZERO)
        .collect()
}

fn build_mt940_tx(
    date_str: &str,
    cd: &str,
    amount_str: &str,
    description: &str,
    ref_no: &str,
) -> ParsedBankTransaction {
    let date = parse_mt940_date(date_str);
    let amount = parse_mt940_amount(amount_str);
    let amount = if cd == "D" { -amount } else { amount };

    let reference_no = if ref_no.is_empty() {
        None
    } else {
        Some(ref_no.trim().to_string())
    };

    let description = if description.trim().is_empty() {
        "MT940 Transaction".to_string()
    } else {
        description.trim().to_string()
    };

    ParsedBankTransaction {
        transaction_date: date,
        description,
        amount,
        currency: "TRY".to_string(),
        balance_after: None,
        reference_no,
    }
}

fn parse_mt940_date(date_str: &str) -> NaiveDate {
    // YYMMDD format
    let year = 2000 + date_str[0..2].parse::<u32>().unwrap_or(0);
    let month = date_str[2..4].parse::<u32>().unwrap_or(1);
    let day = date_str[4..6].parse::<u32>().unwrap_or(1);
    NaiveDate::from_ymd_opt(year as i32, month, day)
        .unwrap_or_else(|| NaiveDate::from_ymd_opt(2000, 1, 1).unwrap())
}

fn parse_mt940_amount(amount_str: &str) -> Decimal {
    let cleaned = amount_str.replace(',', ".");
    cleaned.parse::<Decimal>().unwrap_or(Decimal::ZERO)
}

/// Parse CAMT.053 / ISO 20022 XML format statement data
pub fn parse_camt053(data: &str) -> Vec<ParsedBankTransaction> {
    let mut transactions = Vec::new();

    // Find all entry detail blocks
    for caps in CAMT_TXDTL_RE.captures_iter(data) {
        let block = caps.get(0).map(|m| m.as_str()).unwrap_or("");

        let date = CAMT_DATE_RE
            .captures(block)
            .and_then(|c| c.get(1))
            .and_then(|m| m.as_str().parse::<NaiveDate>().ok())
            .unwrap_or_else(|| chrono::Utc::now().date_naive());

        let (amount, currency) = CAMT_AMOUNT_RE
            .captures(block)
            .map(|c| {
                let currency = c
                    .get(1)
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_else(|| "TRY".to_string());
                let amount_str = c.get(2).map(|m| m.as_str()).unwrap_or("0");
                let amount = amount_str.parse::<Decimal>().unwrap_or(Decimal::ZERO);
                (amount, currency)
            })
            .unwrap_or((Decimal::ZERO, "TRY".to_string()));

        let is_credit = CAMT_CREDIT_DEBIT_RE
            .captures(block)
            .and_then(|c| c.get(1))
            .map(|m| {
                let s = m.as_str().to_lowercase();
                s.starts_with("cr") || s.contains("credit")
            })
            .unwrap_or(true);

        let signed_amount = if is_credit { amount } else { -amount };

        let description = CAMT_RMT_RE
            .captures(block)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string())
            .unwrap_or_else(|| "CAMT.053 Transaction".to_string());

        let reference_no = CAMT_REF_RE
            .captures(block)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().to_string());

        transactions.push(ParsedBankTransaction {
            transaction_date: date,
            description,
            amount: signed_amount,
            currency,
            balance_after: None,
            reference_no,
        });
    }

    transactions
        .into_iter()
        .filter(|t| t.amount != Decimal::ZERO)
        .collect()
}

/// Parse bank-specific XML formats
pub fn parse_bank_xml(bank_code: BankCode, data: &str) -> Vec<ParsedBankTransaction> {
    match bank_code {
        BankCode::IsBankasi => parse_isbank_xml(data),
        BankCode::Garanti => parse_garanti_xml(data),
        BankCode::Halkbank => parse_halkbank_xml(data),
        BankCode::Ziraat => parse_ziraat_xml(data),
        BankCode::YapiKredi => parse_yapikredi_xml(data),
        BankCode::Akbank => parse_akbank_xml(data),
    }
}

fn parse_isbank_xml(data: &str) -> Vec<ParsedBankTransaction> {
    parse_generic_bank_xml(data, "islem", "tarih", "aciklama", "tutar", "TRY")
}

fn parse_garanti_xml(data: &str) -> Vec<ParsedBankTransaction> {
    parse_generic_bank_xml(data, "transaction", "date", "description", "amount", "TRY")
}

fn parse_halkbank_xml(data: &str) -> Vec<ParsedBankTransaction> {
    parse_generic_bank_xml(data, "hareket", "tarih", "aciklama", "tutar", "TRY")
}

fn parse_ziraat_xml(data: &str) -> Vec<ParsedBankTransaction> {
    parse_generic_bank_xml(data, "islem", "tarih", "aciklama", "tutar", "TRY")
}

fn parse_yapikredi_xml(data: &str) -> Vec<ParsedBankTransaction> {
    parse_generic_bank_xml(data, "transaction", "date", "description", "amount", "TRY")
}

fn parse_akbank_xml(data: &str) -> Vec<ParsedBankTransaction> {
    parse_generic_bank_xml(data, "islem", "tarih", "aciklama", "tutar", "TRY")
}

/// Generic XML parser for bank-specific formats
fn parse_generic_bank_xml(
    data: &str,
    tx_tag: &str,
    date_tag: &str,
    desc_tag: &str,
    amount_tag: &str,
    default_currency: &str,
) -> Vec<ParsedBankTransaction> {
    let mut transactions = Vec::new();

    let tx_re = Regex::new(&format!(r"(?s)<{}>(.*?)</{}>", tx_tag, tx_tag))
        .unwrap_or_else(|_| Regex::new(r"(?s)<\w+>(.*?)</\w+>").unwrap());
    let date_re = Regex::new(&format!(r"(?s)<{}>(.*?)</{}>", date_tag, date_tag))
        .unwrap_or_else(|_| Regex::new(r"(?s)<\w+>(.*?)</\w+>").unwrap());
    let desc_re = Regex::new(&format!(r"(?s)<{}>(.*?)</{}>", desc_tag, desc_tag))
        .unwrap_or_else(|_| Regex::new(r"(?s)<\w+>(.*?)</\w+>").unwrap());
    let amount_re = Regex::new(&format!(r"(?s)<{}>(.*?)</{}>", amount_tag, amount_tag))
        .unwrap_or_else(|_| Regex::new(r"(?s)<\w+>(.*?)</\w+>").unwrap());

    for tx_caps in tx_re.captures_iter(data) {
        let block = tx_caps.get(1).map(|m| m.as_str()).unwrap_or("");

        let date = date_re
            .captures(block)
            .and_then(|c| c.get(1))
            .and_then(|m| parse_xml_date(m.as_str()))
            .unwrap_or_else(|| chrono::Utc::now().date_naive());

        let description = desc_re
            .captures(block)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().trim().to_string())
            .unwrap_or_else(|| "XML Transaction".to_string());

        let (amount, _) = amount_re
            .captures(block)
            .and_then(|c| c.get(1))
            .map(|m| {
                let val = m.as_str().trim().replace(',', ".");
                (
                    val.parse::<Decimal>().unwrap_or(Decimal::ZERO),
                    default_currency.to_string(),
                )
            })
            .unwrap_or((Decimal::ZERO, default_currency.to_string()));

        if amount != Decimal::ZERO {
            transactions.push(ParsedBankTransaction {
                transaction_date: date,
                description,
                amount,
                currency: default_currency.to_string(),
                balance_after: None,
                reference_no: None,
            });
        }
    }

    transactions
}

fn parse_xml_date(date_str: &str) -> Option<NaiveDate> {
    let trimmed = date_str.trim();

    // Try ISO format first
    if let Ok(date) = trimmed.parse::<NaiveDate>() {
        return Some(date);
    }

    // Try DD.MM.YYYY
    let parts: Vec<&str> = trimmed.split('.').collect();
    if parts.len() == 3 {
        let day = parts[0].parse::<u32>().ok()?;
        let month = parts[1].parse::<u32>().ok()?;
        let year = parts[2].parse::<i32>().ok()?;
        return NaiveDate::from_ymd_opt(year, month, day);
    }

    // Try DD/MM/YYYY
    let parts: Vec<&str> = trimmed.split('/').collect();
    if parts.len() == 3 {
        let day = parts[0].parse::<u32>().ok()?;
        let month = parts[1].parse::<u32>().ok()?;
        let year = parts[2].parse::<i32>().ok()?;
        return NaiveDate::from_ymd_opt(year, month, day);
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_parse_mt940() {
        let data = r#"
:61:2301010101C1000,00NTRFNONREF//REF123
:86:Invoice payment #12345
:61:2301020102D500,50NTRFNONREF//REF456
:86:Supplier payment
        "#;

        let transactions = parse_mt940(data);
        assert_eq!(transactions.len(), 2);

        assert_eq!(transactions[0].amount, dec!(1000.00));
        assert_eq!(transactions[0].description, "Invoice payment #12345");
        assert_eq!(transactions[0].reference_no, Some("REF123".to_string()));

        assert_eq!(transactions[1].amount, dec!(-500.50));
        assert_eq!(transactions[1].description, "Supplier payment");
        assert_eq!(transactions[1].reference_no, Some("REF456".to_string()));
    }

    #[test]
    fn test_parse_mt940_with_empty_description() {
        let data = ":61:2301010101C250,00NTRFNONREF//REF789";
        let transactions = parse_mt940(data);
        assert_eq!(transactions.len(), 1);
        assert_eq!(transactions[0].amount, dec!(250.00));
        assert_eq!(transactions[0].description, "MT940 Transaction");
    }

    #[test]
    fn test_parse_camt053() {
        let data = r#"
<?xml version="1.0" encoding="UTF-8"?>
<Document>
  <BkToCstmrStmt>
    <Stmt>
      <Ntry>
        <NtryDtls>
          <TxDtls>
            <Amt Ccy="TRY">1500.00</Amt>
            <CdtDbtInd>CRDT</CdtDbtInd>
            <RltdDts>
              <AccptncDtTm>
                <Dt>2023-01-15</Dt>
              </AccptncDtTm>
            </RltdDts>
            <RmtInf>
              <Ustrd>Customer payment INV-2023-001</Ustrd>
            </RmtInf>
            <Refs>
              <EndToEndId>E2E-001</EndToEndId>
            </Refs>
          </TxDtls>
        </NtryDtls>
      </Ntry>
    </Stmt>
  </BkToCstmrStmt>
</Document>
        "#;

        let transactions = parse_camt053(data);
        assert_eq!(transactions.len(), 1);
        assert_eq!(transactions[0].amount, dec!(1500.00));
        assert_eq!(transactions[0].currency, "TRY");
        assert_eq!(transactions[0].description, "Customer payment INV-2023-001");
        assert_eq!(transactions[0].reference_no, Some("E2E-001".to_string()));
    }

    #[test]
    fn test_parse_camt053_debit() {
        let data = r#"
<NtryDtls>
  <TxDtls>
    <Amt Ccy="EUR">750.00</Amt>
    <CdtDbtInd>DBIT</CdtDbtInd>
    <RltdDts><AccptncDtTm><Dt>2023-02-01</Dt></AccptncDtTm></RltdDts>
    <RmtInf><Ustrd>Utility bill payment</Ustrd></RmtInf>
  </TxDtls>
</NtryDtls>
        "#;

        let transactions = parse_camt053(data);
        assert_eq!(transactions.len(), 1);
        assert_eq!(transactions[0].amount, dec!(-750.00));
        assert_eq!(transactions[0].currency, "EUR");
    }

    #[test]
    fn test_parse_bank_xml_isbank() {
        let data = r#"
<root>
  <islem>
    <tarih>15.01.2023</tarih>
    <aciklama>Salary transfer</aciklama>
    <tutar>5000,00</tutar>
  </islem>
  <islem>
    <tarih>16.01.2023</tarih>
    <aciklama>Office rent</aciklama>
    <tutar>-2500,00</tutar>
  </islem>
</root>
        "#;

        let transactions = parse_bank_xml(BankCode::IsBankasi, data);
        assert_eq!(transactions.len(), 2);
        assert_eq!(transactions[0].amount, dec!(5000.00));
        assert_eq!(transactions[0].description, "Salary transfer");
        assert_eq!(transactions[1].amount, dec!(-2500.00));
        assert_eq!(transactions[1].description, "Office rent");
    }

    #[test]
    fn test_parse_bank_xml_garanti() {
        let data = r#"
<root>
  <transaction>
    <date>2023-03-01</date>
    <description>Vendor payment</description>
    <amount>1200.50</amount>
  </transaction>
</root>
        "#;

        let transactions = parse_bank_xml(BankCode::Garanti, data);
        assert_eq!(transactions.len(), 1);
        assert_eq!(transactions[0].amount, dec!(1200.50));
        assert_eq!(transactions[0].description, "Vendor payment");
    }

    #[test]
    fn test_parse_xml_date() {
        assert_eq!(
            parse_xml_date("2023-01-15"),
            Some(NaiveDate::from_ymd_opt(2023, 1, 15).unwrap())
        );
        assert_eq!(
            parse_xml_date("15.01.2023"),
            Some(NaiveDate::from_ymd_opt(2023, 1, 15).unwrap())
        );
        assert_eq!(
            parse_xml_date("15/01/2023"),
            Some(NaiveDate::from_ymd_opt(2023, 1, 15).unwrap())
        );
        assert!(parse_xml_date("invalid").is_none());
    }
}
