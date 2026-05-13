//! Barcode domain models

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Barcode symbology type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default, ToSchema)]
pub enum BarcodeType {
    #[default]
    Ean13,
    Code128,
    QrCode,
    DataMatrix,
}

impl std::fmt::Display for BarcodeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BarcodeType::Ean13 => write!(f, "Ean13"),
            BarcodeType::Code128 => write!(f, "Code128"),
            BarcodeType::QrCode => write!(f, "QrCode"),
            BarcodeType::DataMatrix => write!(f, "DataMatrix"),
        }
    }
}

impl std::str::FromStr for BarcodeType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Ean13" => Ok(BarcodeType::Ean13),
            "Code128" => Ok(BarcodeType::Code128),
            "QrCode" => Ok(BarcodeType::QrCode),
            "DataMatrix" => Ok(BarcodeType::DataMatrix),
            _ => Err(format!("Invalid barcode type: {}", s)),
        }
    }
}

/// Stored barcode configuration
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BarcodeConfig {
    pub id: i64,
    pub tenant_id: i64,
    pub entity_type: String,
    pub entity_id: i64,
    pub barcode_type: BarcodeType,
    pub code: String,
    pub image_data: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Request to create a barcode record
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateBarcode {
    pub entity_type: String,
    pub entity_id: i64,
    pub barcode_type: BarcodeType,
    pub code: String,
}

impl CreateBarcode {
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        if self.entity_type.trim().is_empty() {
            errors.push("Entity type is required".to_string());
        }
        if self.code.trim().is_empty() {
            errors.push("Code is required".to_string());
        }

        match self.barcode_type {
            BarcodeType::Ean13 => {
                let trimmed = self.code.trim();
                if trimmed.len() != 12 && trimmed.len() != 13 {
                    errors.push("EAN-13 code must be 12 or 13 digits".to_string());
                }
                if !trimmed.chars().all(|c| c.is_ascii_digit()) {
                    errors.push("EAN-13 code must contain only digits".to_string());
                }
            }
            BarcodeType::Code128 => {
                if self.code.trim().is_empty() {
                    errors.push("Code128 code must not be empty".to_string());
                }
            }
            BarcodeType::QrCode | BarcodeType::DataMatrix => {
                if self.code.trim().is_empty() {
                    errors.push("QR/DataMatrix code must not be empty".to_string());
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// Barcode response for API consumers
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BarcodeResponse {
    pub id: i64,
    pub entity_type: String,
    pub entity_id: i64,
    pub barcode_type: BarcodeType,
    pub code: String,
    pub image_url: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl From<BarcodeConfig> for BarcodeResponse {
    fn from(b: BarcodeConfig) -> Self {
        Self {
            id: b.id,
            entity_type: b.entity_type,
            entity_id: b.entity_id,
            barcode_type: b.barcode_type,
            code: b.code,
            image_url: b.image_data,
            created_at: b.created_at,
        }
    }
}

/// Request to generate a barcode image
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GenerateBarcodeRequest {
    pub entity_type: String,
    pub entity_type_id: i64,
    pub barcode_type: BarcodeType,
    pub code: String,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

impl GenerateBarcodeRequest {
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let create = CreateBarcode {
            entity_type: self.entity_type.clone(),
            entity_id: self.entity_type_id,
            barcode_type: self.barcode_type.clone(),
            code: self.code.clone(),
        };
        create.validate()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_barcode_type_display() {
        assert_eq!(BarcodeType::Ean13.to_string(), "Ean13");
        assert_eq!(BarcodeType::Code128.to_string(), "Code128");
        assert_eq!(BarcodeType::QrCode.to_string(), "QrCode");
        assert_eq!(BarcodeType::DataMatrix.to_string(), "DataMatrix");
    }

    #[test]
    fn test_barcode_type_from_str() {
        assert_eq!("Ean13".parse::<BarcodeType>().unwrap(), BarcodeType::Ean13);
        assert_eq!(
            "Code128".parse::<BarcodeType>().unwrap(),
            BarcodeType::Code128
        );
        assert_eq!(
            "QrCode".parse::<BarcodeType>().unwrap(),
            BarcodeType::QrCode
        );
        assert_eq!(
            "DataMatrix".parse::<BarcodeType>().unwrap(),
            BarcodeType::DataMatrix
        );
        assert!("Invalid".parse::<BarcodeType>().is_err());
    }

    #[test]
    fn test_create_barcode_validation() {
        let valid_ean = CreateBarcode {
            entity_type: "product".to_string(),
            entity_id: 1,
            barcode_type: BarcodeType::Ean13,
            code: "5901234123457".to_string(),
        };
        assert!(valid_ean.validate().is_ok());

        let invalid_ean = CreateBarcode {
            entity_type: "product".to_string(),
            entity_id: 1,
            barcode_type: BarcodeType::Ean13,
            code: "abc".to_string(),
        };
        assert!(invalid_ean.validate().is_err());

        let empty_type = CreateBarcode {
            entity_type: "".to_string(),
            entity_id: 1,
            barcode_type: BarcodeType::Code128,
            code: "HELLO".to_string(),
        };
        assert!(empty_type.validate().is_err());

        let valid_qr = CreateBarcode {
            entity_type: "invoice".to_string(),
            entity_id: 1,
            barcode_type: BarcodeType::QrCode,
            code: "https://example.com".to_string(),
        };
        assert!(valid_qr.validate().is_ok());
    }

    #[test]
    fn test_barcode_response_from_config() {
        let config = BarcodeConfig {
            id: 1,
            tenant_id: 1,
            entity_type: "product".to_string(),
            entity_id: 42,
            barcode_type: BarcodeType::Ean13,
            code: "5901234123457".to_string(),
            image_data: Some("data:image/png;base64,abc".to_string()),
            created_at: Utc::now(),
        };
        let resp = BarcodeResponse::from(config.clone());
        assert_eq!(resp.id, config.id);
        assert_eq!(resp.entity_type, config.entity_type);
        assert_eq!(resp.entity_id, config.entity_id);
        assert_eq!(resp.barcode_type, config.barcode_type);
        assert_eq!(resp.code, config.code);
        assert_eq!(resp.image_url, config.image_data);
    }
}
