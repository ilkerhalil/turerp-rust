//! Fixed Assets domain models

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

/// Asset status
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum AssetStatus {
    #[default]
    Active,
    InUse,
    UnderMaintenance,
    Disposed,
    WrittenOff,
}

impl std::fmt::Display for AssetStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Active => write!(f, "Active"),
            Self::InUse => write!(f, "InUse"),
            Self::UnderMaintenance => write!(f, "UnderMaintenance"),
            Self::Disposed => write!(f, "Disposed"),
            Self::WrittenOff => write!(f, "WrittenOff"),
        }
    }
}

/// Depreciation method
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum DepreciationMethod {
    #[default]
    StraightLine,
    DecliningBalance,
    UnitsOfProduction,
    None,
}

impl std::fmt::Display for DepreciationMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::StraightLine => write!(f, "StraightLine"),
            Self::DecliningBalance => write!(f, "DecliningBalance"),
            Self::UnitsOfProduction => write!(f, "UnitsOfProduction"),
            Self::None => write!(f, "None"),
        }
    }
}

/// Asset category for grouping assets
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AssetCategory {
    pub id: i64,
    pub tenant_id: i64,
    pub name: String,
    pub description: Option<String>,
    pub default_useful_life_years: i32,
    pub default_depreciation_method: DepreciationMethod,
    pub created_at: DateTime<Utc>,
}

/// Asset entity
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Asset {
    pub id: i64,
    pub tenant_id: i64,
    pub asset_code: String,
    pub name: String,
    pub category_id: Option<i64>,
    pub description: Option<String>,
    pub serial_number: Option<String>,
    pub location: Option<String>,
    pub status: AssetStatus,
    pub acquisition_date: DateTime<Utc>,
    pub acquisition_cost: Decimal,
    pub salvage_value: Decimal,
    pub useful_life_years: i32,
    pub depreciation_method: DepreciationMethod,
    pub accumulated_depreciation: Decimal,
    pub book_value: Decimal,
    pub warranty_expiry: Option<DateTime<Utc>>,
    pub insurance_number: Option<String>,
    pub insurance_expiry: Option<DateTime<Utc>>,
    pub responsible_person_id: Option<i64>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Asset {
    /// Calculate annual depreciation
    pub fn calculate_annual_depreciation(&self) -> Decimal {
        match self.depreciation_method {
            DepreciationMethod::StraightLine => {
                if self.useful_life_years <= 0 {
                    Decimal::ZERO
                } else {
                    (self.acquisition_cost - self.salvage_value)
                        / Decimal::from(self.useful_life_years)
                }
            }
            DepreciationMethod::DecliningBalance => {
                // Double declining balance: 2 * (cost - accumulated) / useful_life
                if self.useful_life_years <= 0 {
                    Decimal::ZERO
                } else {
                    let rate = Decimal::from(2) / Decimal::from(self.useful_life_years);
                    (self.acquisition_cost - self.accumulated_depreciation) * rate
                }
            }
            DepreciationMethod::UnitsOfProduction => {
                // Requires production units - simplified here
                Decimal::ZERO
            }
            DepreciationMethod::None => Decimal::ZERO,
        }
    }

    /// Calculate book value
    pub fn calculate_book_value(&self) -> Decimal {
        self.acquisition_cost - self.accumulated_depreciation
    }
}

/// Maintenance record for an asset
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MaintenanceRecord {
    pub id: i64,
    pub asset_id: i64,
    pub maintenance_date: DateTime<Utc>,
    pub maintenance_type: String,
    pub description: String,
    pub cost: Decimal,
    pub performed_by: Option<String>,
    pub next_maintenance_date: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Create asset request
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct CreateAsset {
    pub tenant_id: i64,
    #[validate(length(min = 1, max = 50, message = "Name must be 1-50 characters"))]
    pub name: String,
    pub category_id: Option<i64>,
    #[validate(length(max = 500, message = "Description must be at most 500 characters"))]
    pub description: Option<String>,
    pub serial_number: Option<String>,
    pub location: Option<String>,
    pub acquisition_date: DateTime<Utc>,
    pub acquisition_cost: Decimal,
    pub salvage_value: Decimal,
    #[validate(range(min = 1, max = 100, message = "Useful life must be 1-100 years"))]
    pub useful_life_years: i32,
    pub depreciation_method: Option<DepreciationMethod>,
    pub warranty_expiry: Option<DateTime<Utc>>,
    pub insurance_number: Option<String>,
    pub insurance_expiry: Option<DateTime<Utc>>,
    pub responsible_person_id: Option<i64>,
    pub notes: Option<String>,
}

impl CreateAsset {
    /// Validate and generate asset code
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        if let Err(e) = validator::Validate::validate(self) {
            for (_, errs) in e.field_errors() {
                for err in errs.iter() {
                    errors.push(err.to_string());
                }
            }
        }

        if self.acquisition_cost < self.salvage_value {
            errors.push(
                "Acquisition cost must be greater than or equal to salvage value".to_string(),
            );
        }

        if self.acquisition_cost < Decimal::ZERO {
            errors.push("Acquisition cost must be non-negative".to_string());
        }

        if self.salvage_value < Decimal::ZERO {
            errors.push("Salvage value must be non-negative".to_string());
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// Update asset request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateAsset {
    pub name: Option<String>,
    pub description: Option<String>,
    pub serial_number: Option<String>,
    pub location: Option<String>,
    pub status: Option<AssetStatus>,
    pub location_id: Option<i64>,
    pub responsible_person_id: Option<i64>,
    pub notes: Option<String>,
}

/// Create maintenance record request
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct CreateMaintenanceRecord {
    pub asset_id: i64,
    pub maintenance_date: DateTime<Utc>,
    #[validate(length(min = 1, max = 100, message = "Type must be 1-100 characters"))]
    pub maintenance_type: String,
    #[validate(length(min = 1, max = 1000, message = "Description must be 1-1000 characters"))]
    pub description: String,
    pub cost: Decimal,
    pub performed_by: Option<String>,
    pub next_maintenance_date: Option<DateTime<Utc>>,
}

impl CreateMaintenanceRecord {
    /// Validate the maintenance record
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        if let Err(e) = validator::Validate::validate(self) {
            for (_, errs) in e.field_errors() {
                for err in errs.iter() {
                    errors.push(err.to_string());
                }
            }
        }

        if self.cost < Decimal::ZERO {
            errors.push("Cost must be non-negative".to_string());
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// Asset response for API
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AssetResponse {
    pub id: i64,
    pub asset_code: String,
    pub name: String,
    pub category_id: Option<i64>,
    pub description: Option<String>,
    pub serial_number: Option<String>,
    pub location: Option<String>,
    pub status: AssetStatus,
    pub acquisition_date: DateTime<Utc>,
    pub acquisition_cost: Decimal,
    pub salvage_value: Decimal,
    pub useful_life_years: i32,
    pub depreciation_method: DepreciationMethod,
    pub accumulated_depreciation: Decimal,
    pub book_value: Decimal,
    pub annual_depreciation: Decimal,
    pub warranty_expiry: Option<DateTime<Utc>>,
    pub insurance_number: Option<String>,
    pub insurance_expiry: Option<DateTime<Utc>>,
    pub responsible_person_id: Option<i64>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Asset> for AssetResponse {
    fn from(asset: Asset) -> Self {
        let annual_depreciation = asset.calculate_annual_depreciation();
        let book_value = asset.calculate_book_value();
        Self {
            id: asset.id,
            asset_code: asset.asset_code,
            name: asset.name,
            category_id: asset.category_id,
            description: asset.description,
            serial_number: asset.serial_number,
            location: asset.location,
            status: asset.status,
            acquisition_date: asset.acquisition_date,
            acquisition_cost: asset.acquisition_cost,
            salvage_value: asset.salvage_value,
            useful_life_years: asset.useful_life_years,
            depreciation_method: asset.depreciation_method,
            accumulated_depreciation: asset.accumulated_depreciation,
            book_value,
            annual_depreciation,
            warranty_expiry: asset.warranty_expiry,
            insurance_number: asset.insurance_number,
            insurance_expiry: asset.insurance_expiry,
            responsible_person_id: asset.responsible_person_id,
            notes: asset.notes,
            created_at: asset.created_at,
            updated_at: asset.updated_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_asset_status_display() {
        assert_eq!(AssetStatus::Active.to_string(), "Active");
        assert_eq!(AssetStatus::InUse.to_string(), "InUse");
        assert_eq!(
            AssetStatus::UnderMaintenance.to_string(),
            "UnderMaintenance"
        );
    }

    #[test]
    fn test_depreciation_calculation() {
        let asset = Asset {
            id: 1,
            tenant_id: 1,
            asset_code: "AST-001".to_string(),
            name: "Test Asset".to_string(),
            category_id: None,
            description: None,
            serial_number: None,
            location: None,
            status: AssetStatus::Active,
            acquisition_date: Utc::now(),
            acquisition_cost: Decimal::from(10000),
            salvage_value: Decimal::from(1000),
            useful_life_years: 5,
            depreciation_method: DepreciationMethod::StraightLine,
            accumulated_depreciation: Decimal::ZERO,
            book_value: Decimal::from(10000),
            warranty_expiry: None,
            insurance_number: None,
            insurance_expiry: None,
            responsible_person_id: None,
            notes: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let annual_dep = asset.calculate_annual_depreciation();
        // (10000 - 1000) / 5 = 1800
        assert_eq!(annual_dep, Decimal::from(1800));
    }

    #[test]
    fn test_create_asset_validation() {
        let valid = CreateAsset {
            tenant_id: 1,
            name: "Test Asset".to_string(),
            category_id: None,
            description: None,
            serial_number: None,
            location: None,
            acquisition_date: Utc::now(),
            acquisition_cost: Decimal::from(10000),
            salvage_value: Decimal::from(1000),
            useful_life_years: 5,
            depreciation_method: Some(DepreciationMethod::StraightLine),
            warranty_expiry: None,
            insurance_number: None,
            insurance_expiry: None,
            responsible_person_id: None,
            notes: None,
        };
        assert!(valid.validate().is_ok());

        let invalid = CreateAsset {
            tenant_id: 1,
            name: "".to_string(),
            category_id: None,
            description: None,
            serial_number: None,
            location: None,
            acquisition_date: Utc::now(),
            acquisition_cost: Decimal::from(100),
            salvage_value: Decimal::from(1000), // Invalid: salvage > cost
            useful_life_years: 5,
            depreciation_method: Some(DepreciationMethod::StraightLine),
            warranty_expiry: None,
            insurance_number: None,
            insurance_expiry: None,
            responsible_person_id: None,
            notes: None,
        };
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn test_declining_balance_depreciation() {
        let asset = Asset {
            id: 1,
            tenant_id: 1,
            asset_code: "AST-002".to_string(),
            name: "Test Asset".to_string(),
            category_id: None,
            description: None,
            serial_number: None,
            location: None,
            status: AssetStatus::Active,
            acquisition_date: Utc::now(),
            acquisition_cost: Decimal::from(10000),
            salvage_value: Decimal::from(1000),
            useful_life_years: 5,
            depreciation_method: DepreciationMethod::DecliningBalance,
            accumulated_depreciation: Decimal::ZERO,
            book_value: Decimal::from(10000),
            warranty_expiry: None,
            insurance_number: None,
            insurance_expiry: None,
            responsible_person_id: None,
            notes: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let annual_dep = asset.calculate_annual_depreciation();
        // 2/5 * 10000 = 4000
        assert_eq!(annual_dep, Decimal::from(4000));
    }

    #[test]
    fn test_negative_values_rejected() {
        let invalid_cost = CreateAsset {
            tenant_id: 1,
            name: "Test Asset".to_string(),
            category_id: None,
            description: None,
            serial_number: None,
            location: None,
            acquisition_date: Utc::now(),
            acquisition_cost: Decimal::from(-1000), // Negative
            salvage_value: Decimal::from(100),
            useful_life_years: 5,
            depreciation_method: Some(DepreciationMethod::StraightLine),
            warranty_expiry: None,
            insurance_number: None,
            insurance_expiry: None,
            responsible_person_id: None,
            notes: None,
        };
        assert!(invalid_cost.validate().is_err());

        let invalid_salvage = CreateAsset {
            tenant_id: 1,
            name: "Test Asset".to_string(),
            category_id: None,
            description: None,
            serial_number: None,
            location: None,
            acquisition_date: Utc::now(),
            acquisition_cost: Decimal::from(1000),
            salvage_value: Decimal::from(-100), // Negative
            useful_life_years: 5,
            depreciation_method: Some(DepreciationMethod::StraightLine),
            warranty_expiry: None,
            insurance_number: None,
            insurance_expiry: None,
            responsible_person_id: None,
            notes: None,
        };
        assert!(invalid_salvage.validate().is_err());
    }
}
