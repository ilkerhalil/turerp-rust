//! e-Fatura domain models
//!
//! Provides types for Turkish e-Fatura (electronic invoicing) integration
//! with GIB (Gelir İdaresi Başkanlığı), including UBL-TR profile types,
//! party information, line items, and document status tracking.

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// e-Fatura document status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub enum EFaturaStatus {
    Draft,
    Signed,
    Sent,
    Accepted,
    Rejected,
    Cancelled,
    Error,
}

impl std::fmt::Display for EFaturaStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EFaturaStatus::Draft => write!(f, "Draft"),
            EFaturaStatus::Signed => write!(f, "Signed"),
            EFaturaStatus::Sent => write!(f, "Sent"),
            EFaturaStatus::Accepted => write!(f, "Accepted"),
            EFaturaStatus::Rejected => write!(f, "Rejected"),
            EFaturaStatus::Cancelled => write!(f, "Cancelled"),
            EFaturaStatus::Error => write!(f, "Error"),
        }
    }
}

impl std::str::FromStr for EFaturaStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Draft" => Ok(EFaturaStatus::Draft),
            "Signed" => Ok(EFaturaStatus::Signed),
            "Sent" => Ok(EFaturaStatus::Sent),
            "Accepted" => Ok(EFaturaStatus::Accepted),
            "Rejected" => Ok(EFaturaStatus::Rejected),
            "Cancelled" => Ok(EFaturaStatus::Cancelled),
            "Error" => Ok(EFaturaStatus::Error),
            _ => Err(format!("Invalid e-Fatura status: {}", s)),
        }
    }
}

/// e-Fatura profile type (UBL-TR profile)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub enum EFaturaProfile {
    TemelFatura,
    Ihracat,
    YolcuBeleni,
    OzelMatbuFatura,
}

impl std::fmt::Display for EFaturaProfile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EFaturaProfile::TemelFatura => write!(f, "TEMELFATURA"),
            EFaturaProfile::Ihracat => write!(f, "IHRACAT"),
            EFaturaProfile::YolcuBeleni => write!(f, "YOLCUBELENI"),
            EFaturaProfile::OzelMatbuFatura => write!(f, "OZELMATBU"),
        }
    }
}

impl std::str::FromStr for EFaturaProfile {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "TEMELFATURA" => Ok(EFaturaProfile::TemelFatura),
            "IHRACAT" => Ok(EFaturaProfile::Ihracat),
            "YOLCUBELENI" => Ok(EFaturaProfile::YolcuBeleni),
            "OZELMATBU" => Ok(EFaturaProfile::OzelMatbuFatura),
            _ => Err(format!("Invalid e-Fatura profile: {}", s)),
        }
    }
}

/// Address information
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AddressInfo {
    pub street: String,
    pub district: Option<String>,
    pub city: String,
    pub country: Option<String>,
    pub postal_code: Option<String>,
}

/// Party information (sender/receiver)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PartyInfo {
    pub vkn_tckn: String,
    pub name: String,
    pub tax_office: String,
    pub address: AddressInfo,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub register_number: Option<String>,
    pub mersis_number: Option<String>,
}

/// e-Fatura line item
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EFaturaLine {
    pub id: String,
    pub product_name: String,
    pub quantity: Decimal,
    pub unit: String,
    pub unit_price: Decimal,
    pub line_amount: Decimal,
    pub tax_rate: Decimal,
    pub tax_amount: Decimal,
}

/// Tax subtotal for e-Fatura
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TaxSubtotal {
    pub tax_type: String,
    pub taxable_amount: Decimal,
    pub tax_amount: Decimal,
    pub rate: Decimal,
}

/// Legal monetary total
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MonetaryTotal {
    pub line_extension_amount: Decimal,
    pub tax_exclusive_amount: Decimal,
    pub tax_inclusive_amount: Decimal,
    pub allowance_total_amount: Option<Decimal>,
    pub payable_amount: Decimal,
}

/// e-Fatura document
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EFatura {
    pub id: i64,
    pub tenant_id: i64,
    pub invoice_id: Option<i64>,
    pub uuid: String,
    pub document_number: String,
    pub issue_date: NaiveDate,
    pub profile_id: EFaturaProfile,
    pub sender: PartyInfo,
    pub receiver: PartyInfo,
    pub lines: Vec<EFaturaLine>,
    pub tax_totals: Vec<TaxSubtotal>,
    pub legal_monetary_total: MonetaryTotal,
    pub status: EFaturaStatus,
    pub response_code: Option<String>,
    pub response_desc: Option<String>,
    pub xml_content: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Create e-Fatura from an invoice
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateEFatura {
    pub invoice_id: i64,
    pub profile_id: EFaturaProfile,
}

/// e-Fatura response DTO
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EFaturaResponse {
    pub id: i64,
    pub tenant_id: i64,
    pub invoice_id: Option<i64>,
    pub uuid: String,
    pub document_number: String,
    pub issue_date: NaiveDate,
    pub profile_id: EFaturaProfile,
    pub status: EFaturaStatus,
    pub response_code: Option<String>,
    pub response_desc: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<EFatura> for EFaturaResponse {
    fn from(f: EFatura) -> Self {
        Self {
            id: f.id,
            tenant_id: f.tenant_id,
            invoice_id: f.invoice_id,
            uuid: f.uuid,
            document_number: f.document_number,
            issue_date: f.issue_date,
            profile_id: f.profile_id,
            status: f.status,
            response_code: f.response_code,
            response_desc: f.response_desc,
            created_at: f.created_at,
            updated_at: f.updated_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_efatura_status_display() {
        assert_eq!(EFaturaStatus::Draft.to_string(), "Draft");
        assert_eq!(EFaturaStatus::Signed.to_string(), "Signed");
        assert_eq!(EFaturaStatus::Sent.to_string(), "Sent");
        assert_eq!(EFaturaStatus::Accepted.to_string(), "Accepted");
        assert_eq!(EFaturaStatus::Rejected.to_string(), "Rejected");
        assert_eq!(EFaturaStatus::Cancelled.to_string(), "Cancelled");
        assert_eq!(EFaturaStatus::Error.to_string(), "Error");
    }

    #[test]
    fn test_efatura_status_from_str() {
        assert_eq!(
            "Draft".parse::<EFaturaStatus>().unwrap(),
            EFaturaStatus::Draft
        );
        assert_eq!(
            "Signed".parse::<EFaturaStatus>().unwrap(),
            EFaturaStatus::Signed
        );
        assert_eq!(
            "Sent".parse::<EFaturaStatus>().unwrap(),
            EFaturaStatus::Sent
        );
        assert_eq!(
            "Accepted".parse::<EFaturaStatus>().unwrap(),
            EFaturaStatus::Accepted
        );
        assert_eq!(
            "Rejected".parse::<EFaturaStatus>().unwrap(),
            EFaturaStatus::Rejected
        );
        assert_eq!(
            "Cancelled".parse::<EFaturaStatus>().unwrap(),
            EFaturaStatus::Cancelled
        );
        assert_eq!(
            "Error".parse::<EFaturaStatus>().unwrap(),
            EFaturaStatus::Error
        );
        assert!("INVALID".parse::<EFaturaStatus>().is_err());
    }

    #[test]
    fn test_efatura_profile_display() {
        assert_eq!(EFaturaProfile::TemelFatura.to_string(), "TEMELFATURA");
        assert_eq!(EFaturaProfile::Ihracat.to_string(), "IHRACAT");
        assert_eq!(EFaturaProfile::YolcuBeleni.to_string(), "YOLCUBELENI");
        assert_eq!(EFaturaProfile::OzelMatbuFatura.to_string(), "OZELMATBU");
    }

    #[test]
    fn test_efatura_profile_from_str() {
        assert_eq!(
            "TEMELFATURA".parse::<EFaturaProfile>().unwrap(),
            EFaturaProfile::TemelFatura
        );
        assert_eq!(
            "IHRACAT".parse::<EFaturaProfile>().unwrap(),
            EFaturaProfile::Ihracat
        );
        assert_eq!(
            "YOLCUBELENI".parse::<EFaturaProfile>().unwrap(),
            EFaturaProfile::YolcuBeleni
        );
        assert_eq!(
            "OZELMATBU".parse::<EFaturaProfile>().unwrap(),
            EFaturaProfile::OzelMatbuFatura
        );
        // Case-insensitive
        assert_eq!(
            "temelfatura".parse::<EFaturaProfile>().unwrap(),
            EFaturaProfile::TemelFatura
        );
        assert!("INVALID".parse::<EFaturaProfile>().is_err());
    }

    #[test]
    fn test_efatura_response_from_efatura() {
        let fatura = EFatura {
            id: 1,
            tenant_id: 100,
            invoice_id: Some(42),
            uuid: "uuid-123".to_string(),
            document_number: "FTR-2024-001".to_string(),
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
            lines: vec![],
            tax_totals: vec![],
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
        };

        let response = EFaturaResponse::from(fatura);
        assert_eq!(response.id, 1);
        assert_eq!(response.tenant_id, 100);
        assert_eq!(response.invoice_id, Some(42));
        assert_eq!(response.uuid, "uuid-123");
        assert_eq!(response.document_number, "FTR-2024-001");
        assert_eq!(response.profile_id, EFaturaProfile::TemelFatura);
        assert_eq!(response.status, EFaturaStatus::Draft);
    }
}
