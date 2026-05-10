//! CAMT.053 / ISO 20022 statement parser
//!
//! Parses Cash Management message format (CAMT.053) XML bank statements
//! into domain model types.

use chrono::{DateTime, NaiveDate, Utc};
use quick_xml::de::from_str;
use rust_decimal::Decimal;
use serde::Deserialize;

use crate::domain::bank::model::{CamtEntry, CamtStatement};
use crate::error::ApiError;

/// Parse a CAMT.053 XML string into a `CamtStatement`
pub fn parse_camt053(xml: &str) -> Result<CamtStatement, ApiError> {
    let document: Document = from_str(xml)
        .map_err(|e| ApiError::Validation(format!("Failed to parse CAMT.053 XML: {}", e)))?;

    let stmt = document.bk_to_cstmr_stmt.stmt;

    let statement_id = stmt.id;
    let creation_date = parse_datetime(&stmt.cre_dt_tm).unwrap_or_else(|_| Utc::now());

    let account_iban = stmt.acct.id.iban.unwrap_or_default();

    let mut entries = Vec::new();
    for ntry in stmt.ntry.unwrap_or_default() {
        if let Some(entry) = convert_entry(&ntry) {
            entries.push(entry);
        }
    }

    Ok(CamtStatement {
        statement_id,
        creation_date,
        account_iban,
        entries,
    })
}

fn convert_entry(ntry: &Ntry) -> Option<CamtEntry> {
    let amount = ntry.amt.value.parse::<Decimal>().ok()?;
    let currency = ntry.amt.ccy.clone();
    let credit_debit = ntry.cdt_dbt_ind.clone();

    let entry_date = ntry
        .val_dt
        .as_ref()
        .and_then(|v| v.dt.as_ref())
        .and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok())
        .or_else(|| {
            ntry.bookg_dt
                .as_ref()
                .and_then(|b| b.dt.as_ref())
                .and_then(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").ok())
        })
        .unwrap_or_else(|| Utc::now().date_naive());

    let reference = ntry.ntry_ref.clone();

    let (description, counterparty_name, counterparty_iban) =
        ntry.ntry_dtls
            .as_ref()
            .map_or((None, None, None), |details| {
                let desc = details.tx_dtls.as_ref().and_then(|tx| {
                    tx.rmt_inf
                        .as_ref()
                        .and_then(|r| r.ustrd.clone())
                        .or_else(|| tx.addtl_ntry_inf.clone())
                });

                let (cp_name, cp_iban) = details
                    .tx_dtls
                    .as_ref()
                    .and_then(|tx| tx.rltd_pties.as_ref())
                    .map_or((None, None), |rp| {
                        let name = rp.cdtr.as_ref().and_then(|c| c.nm.clone());
                        let iban = rp.cdtr_acct.as_ref().and_then(|a| a.id.iban.clone());
                        (name, iban)
                    });

                (desc, cp_name, cp_iban)
            });

    Some(CamtEntry {
        entry_date,
        amount,
        currency,
        credit_debit,
        reference,
        description,
        counterparty_name,
        counterparty_iban,
    })
}

fn parse_datetime(s: &str) -> Result<DateTime<Utc>, ApiError> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .or_else(|_| {
            NaiveDate::parse_from_str(s, "%Y-%m-%d")
                .map(|d| d.and_hms_opt(0, 0, 0).unwrap_or_default())
                .map(|dt| DateTime::from_naive_utc_and_offset(dt, Utc))
        })
        .map_err(|e| ApiError::Validation(format!("Invalid datetime: {} - {}", s, e)))
}

// --- Serde structs for CAMT.053 XML ---

#[derive(Debug, Deserialize)]
struct Document {
    #[serde(rename = "BkToCstmrStmt")]
    bk_to_cstmr_stmt: BkToCstmrStmt,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct BkToCstmrStmt {
    #[serde(rename = "GrpHdr", default)]
    grp_hdr: Option<GrpHdr>,
    #[serde(rename = "Stmt")]
    stmt: Stmt,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct GrpHdr {
    #[serde(rename = "MsgId")]
    msg_id: String,
    #[serde(rename = "CreDtTm")]
    cre_dt_tm: String,
}

#[derive(Debug, Deserialize)]
struct Stmt {
    #[serde(rename = "Id")]
    id: String,
    #[serde(rename = "CreDtTm")]
    cre_dt_tm: String,
    #[serde(rename = "Acct")]
    acct: Acct,
    #[serde(rename = "Ntry", default)]
    ntry: Option<Vec<Ntry>>,
}

#[derive(Debug, Deserialize)]
struct Acct {
    #[serde(rename = "Id")]
    id: AcctId,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct AcctId {
    #[serde(rename = "IBAN")]
    iban: Option<String>,
    #[serde(rename = "Othr")]
    othr: Option<OthrId>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct OthrId {
    #[serde(rename = "Id")]
    id: String,
}

#[derive(Debug, Deserialize)]
struct Ntry {
    #[serde(rename = "Amt")]
    amt: Amt,
    #[serde(rename = "CdtDbtInd")]
    cdt_dbt_ind: String,
    #[serde(rename = "ValDt")]
    val_dt: Option<ValDt>,
    #[serde(rename = "BookgDt")]
    bookg_dt: Option<BookgDt>,
    #[serde(rename = "NtryDtls")]
    ntry_dtls: Option<NtryDtls>,
    #[serde(rename = "NtryRef")]
    ntry_ref: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Amt {
    #[serde(rename = "@Ccy")]
    ccy: String,
    #[serde(rename = "$text")]
    value: String,
}

#[derive(Debug, Deserialize)]
struct ValDt {
    #[serde(rename = "Dt")]
    dt: Option<String>,
}

#[derive(Debug, Deserialize)]
struct BookgDt {
    #[serde(rename = "Dt")]
    dt: Option<String>,
}

#[derive(Debug, Deserialize)]
struct NtryDtls {
    #[serde(rename = "TxDtls")]
    tx_dtls: Option<TxDtls>,
}

#[derive(Debug, Deserialize)]
struct TxDtls {
    #[serde(rename = "RmtInf")]
    rmt_inf: Option<RmtInf>,
    #[serde(rename = "RltdPties")]
    rltd_pties: Option<RltdPties>,
    #[serde(rename = "AddtlNtryInf")]
    addtl_ntry_inf: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RmtInf {
    #[serde(rename = "Ustrd")]
    ustrd: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RltdPties {
    #[serde(rename = "Cdtr")]
    cdtr: Option<Cdtr>,
    #[serde(rename = "CdtrAcct")]
    cdtr_acct: Option<CdtrAcct>,
}

#[derive(Debug, Deserialize)]
struct Cdtr {
    #[serde(rename = "Nm")]
    nm: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CdtrAcct {
    #[serde(rename = "Id")]
    id: CdtrAcctId,
}

#[derive(Debug, Deserialize)]
struct CdtrAcctId {
    #[serde(rename = "IBAN")]
    iban: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn sample_camt053() -> String {
        r#"<?xml version="1.0" encoding="UTF-8"?>
<Document xmlns="urn:iso:std:iso:20022:tech:xsd:camt.053.001.02">
  <BkToCstmrStmt>
    <GrpHdr>
      <MsgId>MSG-001</MsgId>
      <CreDtTm>2024-01-15T10:30:00Z</CreDtTm>
    </GrpHdr>
    <Stmt>
      <Id>STMT-001</Id>
      <CreDtTm>2024-01-15T10:30:00Z</CreDtTm>
      <Acct>
        <Id>
          <IBAN>TR000123456789012345678901</IBAN>
        </Id>
      </Acct>
      <Ntry>
        <Amt Ccy="TRY">1500.00</Amt>
        <CdtDbtInd>CRDT</CdtDbtInd>
        <ValDt>
          <Dt>2024-01-15</Dt>
        </ValDt>
        <NtryDtls>
          <TxDtls>
            <RmtInf>
              <Ustrd>Invoice payment #12345</Ustrd>
            </RmtInf>
            <RltdPties>
              <Cdtr>
                <Nm>ABC Company</Nm>
              </Cdtr>
              <CdtrAcct>
                <Id>
                  <IBAN>TR009876543210987654321098</IBAN>
                </Id>
              </CdtrAcct>
            </RltdPties>
          </TxDtls>
        </NtryDtls>
        <NtryRef>REF-001</NtryRef>
      </Ntry>
      <Ntry>
        <Amt Ccy="TRY">2500.00</Amt>
        <CdtDbtInd>DBIT</CdtDbtInd>
        <ValDt>
          <Dt>2024-01-14</Dt>
        </ValDt>
        <NtryDtls>
          <TxDtls>
            <AddtlNtryInf>Utility payment</AddtlNtryInf>
          </TxDtls>
        </NtryDtls>
        <NtryRef>REF-002</NtryRef>
      </Ntry>
    </Stmt>
  </BkToCstmrStmt>
</Document>"#
            .to_string()
    }

    #[test]
    fn test_parse_camt053_success() {
        let xml = sample_camt053();
        let result = parse_camt053(&xml);
        assert!(result.is_ok(), "Parse failed: {:?}", result.err());

        let stmt = result.unwrap();
        assert_eq!(stmt.statement_id, "STMT-001");
        assert_eq!(stmt.account_iban, "TR000123456789012345678901");
        assert_eq!(stmt.entries.len(), 2);
    }

    #[test]
    fn test_parse_camt053_entry_details() {
        let xml = sample_camt053();
        let stmt = parse_camt053(&xml).unwrap();

        let first = &stmt.entries[0];
        assert_eq!(first.amount, dec!(1500.00));
        assert_eq!(first.currency, "TRY");
        assert_eq!(first.credit_debit, "CRDT");
        assert_eq!(
            first.entry_date,
            NaiveDate::from_ymd_opt(2024, 1, 15).unwrap()
        );
        assert_eq!(first.reference.as_ref().unwrap(), "REF-001");
        assert_eq!(
            first.description.as_ref().unwrap(),
            "Invoice payment #12345"
        );
        assert_eq!(first.counterparty_name.as_ref().unwrap(), "ABC Company");
        assert_eq!(
            first.counterparty_iban.as_ref().unwrap(),
            "TR009876543210987654321098"
        );
    }

    #[test]
    fn test_parse_camt053_second_entry() {
        let xml = sample_camt053();
        let stmt = parse_camt053(&xml).unwrap();

        let second = &stmt.entries[1];
        assert_eq!(second.amount, dec!(2500.00));
        assert_eq!(second.credit_debit, "DBIT");
        assert_eq!(
            second.entry_date,
            NaiveDate::from_ymd_opt(2024, 1, 14).unwrap()
        );
        assert_eq!(second.description.as_ref().unwrap(), "Utility payment");
    }

    #[test]
    fn test_parse_camt053_invalid_xml() {
        let xml = "not valid xml";
        let result = parse_camt053(xml);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_camt053_minimal() {
        let xml = r#"<?xml version="1.0"?>
<Document>
  <BkToCstmrStmt>
    <Stmt>
      <Id>MINI-001</Id>
      <CreDtTm>2024-01-01</CreDtTm>
      <Acct>
        <Id>
          <IBAN>TR001111111111111111111111</IBAN>
        </Id>
      </Acct>
    </Stmt>
  </BkToCstmrStmt>
</Document>"#;

        let stmt = parse_camt053(xml).unwrap();
        assert_eq!(stmt.statement_id, "MINI-001");
        assert_eq!(stmt.account_iban, "TR001111111111111111111111");
        assert!(stmt.entries.is_empty());
    }
}
