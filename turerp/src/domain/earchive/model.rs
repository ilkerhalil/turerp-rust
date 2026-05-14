//! E-Archive domain models
//!
//! Provides types for Turkish e-Arşiv Fatura and E-Serbest Meslek Makbuzu
//! (electronic tax compliance documents), including document status tracking.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// E-Archive document type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub enum EarchiveType {
    EArchiveInvoice,
    ESerbestMeslekMakbuzu,
}

impl std::fmt::Display for EarchiveType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EarchiveType::EArchiveInvoice => write!(f, "EArchiveInvoice"),
            EarchiveType::ESerbestMeslekMakbuzu => write!(f, "ESerbestMeslekMakbuzu"),
        }
    }
}

impl std::str::FromStr for EarchiveType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "EArchiveInvoice" => Ok(EarchiveType::EArchiveInvoice),
            "ESerbestMeslekMakbuzu" => Ok(EarchiveType::ESerbestMeslekMakbuzu),
            _ => Err(format!("Invalid e-archive type: {}", s)),
        }
    }
}

/// E-Archive document status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub enum EarchiveStatus {
    Draft,
    Generated,
    Signed,
    Sent,
    Accepted,
    Rejected,
    Cancelled,
}

impl std::fmt::Display for EarchiveStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EarchiveStatus::Draft => write!(f, "Draft"),
            EarchiveStatus::Generated => write!(f, "Generated"),
            EarchiveStatus::Signed => write!(f, "Signed"),
            EarchiveStatus::Sent => write!(f, "Sent"),
            EarchiveStatus::Accepted => write!(f, "Accepted"),
            EarchiveStatus::Rejected => write!(f, "Rejected"),
            EarchiveStatus::Cancelled => write!(f, "Cancelled"),
        }
    }
}

impl std::str::FromStr for EarchiveStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Draft" => Ok(EarchiveStatus::Draft),
            "Generated" => Ok(EarchiveStatus::Generated),
            "Signed" => Ok(EarchiveStatus::Signed),
            "Sent" => Ok(EarchiveStatus::Sent),
            "Accepted" => Ok(EarchiveStatus::Accepted),
            "Rejected" => Ok(EarchiveStatus::Rejected),
            "Cancelled" => Ok(EarchiveStatus::Cancelled),
            _ => Err(format!("Invalid e-archive status: {}", s)),
        }
    }
}

/// E-Archive document entity
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EarchiveDocument {
    pub id: i64,
    pub tenant_id: i64,
    pub document_type: EarchiveType,
    pub related_invoice_id: Option<i64>,
    pub uuid: String,
    pub xml_content: String,
    pub signature: Option<String>,
    pub status: EarchiveStatus,
    pub gib_response: Option<String>,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub sent_at: Option<DateTime<Utc>>,
}

/// Create e-archive document request
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateEarchiveDocument {
    pub document_type: EarchiveType,
    pub related_invoice_id: Option<i64>,
    pub xml_content: String,
}

/// E-Archive response DTO
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EarchiveResponse {
    pub id: i64,
    pub tenant_id: i64,
    pub document_type: EarchiveType,
    pub uuid: String,
    pub status: EarchiveStatus,
    pub created_at: DateTime<Utc>,
    pub sent_at: Option<DateTime<Utc>>,
}

impl From<EarchiveDocument> for EarchiveResponse {
    fn from(doc: EarchiveDocument) -> Self {
        Self {
            id: doc.id,
            tenant_id: doc.tenant_id,
            document_type: doc.document_type,
            uuid: doc.uuid,
            status: doc.status,
            created_at: doc.created_at,
            sent_at: doc.sent_at,
        }
    }
}

/// Generate e-archive from invoice request
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct GenerateEarchiveRequest {
    pub invoice_id: i64,
    pub document_type: EarchiveType,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_earchive_type_display() {
        assert_eq!(EarchiveType::EArchiveInvoice.to_string(), "EArchiveInvoice");
        assert_eq!(
            EarchiveType::ESerbestMeslekMakbuzu.to_string(),
            "ESerbestMeslekMakbuzu"
        );
    }

    #[test]
    fn test_earchive_type_from_str() {
        assert_eq!(
            "EArchiveInvoice".parse::<EarchiveType>().unwrap(),
            EarchiveType::EArchiveInvoice
        );
        assert_eq!(
            "ESerbestMeslekMakbuzu".parse::<EarchiveType>().unwrap(),
            EarchiveType::ESerbestMeslekMakbuzu
        );
        assert!("INVALID".parse::<EarchiveType>().is_err());
    }

    #[test]
    fn test_earchive_status_display() {
        assert_eq!(EarchiveStatus::Draft.to_string(), "Draft");
        assert_eq!(EarchiveStatus::Generated.to_string(), "Generated");
        assert_eq!(EarchiveStatus::Signed.to_string(), "Signed");
        assert_eq!(EarchiveStatus::Sent.to_string(), "Sent");
        assert_eq!(EarchiveStatus::Accepted.to_string(), "Accepted");
        assert_eq!(EarchiveStatus::Rejected.to_string(), "Rejected");
        assert_eq!(EarchiveStatus::Cancelled.to_string(), "Cancelled");
    }

    #[test]
    fn test_earchive_status_from_str() {
        assert_eq!(
            "Draft".parse::<EarchiveStatus>().unwrap(),
            EarchiveStatus::Draft
        );
        assert_eq!(
            "Generated".parse::<EarchiveStatus>().unwrap(),
            EarchiveStatus::Generated
        );
        assert_eq!(
            "Signed".parse::<EarchiveStatus>().unwrap(),
            EarchiveStatus::Signed
        );
        assert_eq!(
            "Sent".parse::<EarchiveStatus>().unwrap(),
            EarchiveStatus::Sent
        );
        assert_eq!(
            "Accepted".parse::<EarchiveStatus>().unwrap(),
            EarchiveStatus::Accepted
        );
        assert_eq!(
            "Rejected".parse::<EarchiveStatus>().unwrap(),
            EarchiveStatus::Rejected
        );
        assert_eq!(
            "Cancelled".parse::<EarchiveStatus>().unwrap(),
            EarchiveStatus::Cancelled
        );
        assert!("INVALID".parse::<EarchiveStatus>().is_err());
    }

    #[test]
    fn test_earchive_response_from_document() {
        let doc = EarchiveDocument {
            id: 1,
            tenant_id: 100,
            document_type: EarchiveType::EArchiveInvoice,
            related_invoice_id: Some(42),
            uuid: "uuid-123".to_string(),
            xml_content: "<xml/>".to_string(),
            signature: None,
            status: EarchiveStatus::Draft,
            gib_response: None,
            error_message: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            sent_at: None,
        };

        let response = EarchiveResponse::from(doc);
        assert_eq!(response.id, 1);
        assert_eq!(response.tenant_id, 100);
        assert_eq!(response.document_type, EarchiveType::EArchiveInvoice);
        assert_eq!(response.uuid, "uuid-123");
        assert_eq!(response.status, EarchiveStatus::Draft);
    }
}
