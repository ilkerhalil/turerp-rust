//! Row-level validation for bulk import entities

use rust_decimal::Decimal;

use crate::common::import::model::{
    CariImportRow, ChartAccountImportRow, ImportError, ProductImportRow, StockMovementImportRow,
};
use crate::domain::accounting::model::AccountType;
use crate::domain::cari::model::CariType;
use crate::domain::stock::model::MovementType;

/// Validate a product import row
pub fn validate_product_row(row: usize, data: &ProductImportRow) -> Vec<ImportError> {
    let mut errors = Vec::new();

    if data.code.trim().is_empty() {
        errors.push(ImportError {
            row,
            field: Some("code".to_string()),
            message: "Product code is required".to_string(),
        });
    }
    if data.name.trim().is_empty() {
        errors.push(ImportError {
            row,
            field: Some("name".to_string()),
            message: "Product name is required".to_string(),
        });
    }
    if data.unit_price.trim().is_empty() {
        errors.push(ImportError {
            row,
            field: Some("unit_price".to_string()),
            message: "Unit price is required".to_string(),
        });
    } else {
        match parse_decimal(&data.unit_price) {
            Ok(price) if price < Decimal::ZERO => {
                errors.push(ImportError {
                    row,
                    field: Some("unit_price".to_string()),
                    message: "Unit price cannot be negative".to_string(),
                });
            }
            Ok(_) => {}
            Err(_) => {
                errors.push(ImportError {
                    row,
                    field: Some("unit_price".to_string()),
                    message: "Unit price must be a valid number".to_string(),
                });
            }
        }
    }

    errors
}

/// Validate a cari import row
pub fn validate_cari_row(row: usize, data: &CariImportRow) -> Vec<ImportError> {
    let mut errors = Vec::new();

    if data.code.trim().is_empty() {
        errors.push(ImportError {
            row,
            field: Some("code".to_string()),
            message: "Cari code is required".to_string(),
        });
    }
    if data.name.trim().is_empty() {
        errors.push(ImportError {
            row,
            field: Some("name".to_string()),
            message: "Cari name is required".to_string(),
        });
    }
    if data.cari_type.trim().is_empty() {
        errors.push(ImportError {
            row,
            field: Some("cari_type".to_string()),
            message: "Cari type is required".to_string(),
        });
    } else if data.cari_type.parse::<CariType>().is_err() {
        errors.push(ImportError {
            row,
            field: Some("cari_type".to_string()),
            message: format!(
                "Invalid cari type '{}'. Valid values: customer, vendor, both",
                data.cari_type
            ),
        });
    }

    if let Some(ref email) = data.email {
        if !email.is_empty() && !is_valid_email(email) {
            errors.push(ImportError {
                row,
                field: Some("email".to_string()),
                message: "Invalid email format".to_string(),
            });
        }
    }

    errors
}

/// Validate a chart of accounts import row
pub fn validate_chart_account_row(row: usize, data: &ChartAccountImportRow) -> Vec<ImportError> {
    let mut errors = Vec::new();

    if data.code.trim().is_empty() {
        errors.push(ImportError {
            row,
            field: Some("code".to_string()),
            message: "Account code is required".to_string(),
        });
    }
    if data.name.trim().is_empty() {
        errors.push(ImportError {
            row,
            field: Some("name".to_string()),
            message: "Account name is required".to_string(),
        });
    }
    if data.account_type.trim().is_empty() {
        errors.push(ImportError {
            row,
            field: Some("account_type".to_string()),
            message: "Account type is required".to_string(),
        });
    } else if data.account_type.parse::<AccountType>().is_err() {
        errors.push(ImportError {
            row,
            field: Some("account_type".to_string()),
            message: format!(
                "Invalid account type '{}'. Valid values: Asset, Liability, Equity, Revenue, Expense",
                data.account_type
            ),
        });
    }

    errors
}

/// Validate a stock movement import row
pub fn validate_stock_movement_row(row: usize, data: &StockMovementImportRow) -> Vec<ImportError> {
    let mut errors = Vec::new();

    if data.product_code.trim().is_empty() {
        errors.push(ImportError {
            row,
            field: Some("product_code".to_string()),
            message: "Product code is required".to_string(),
        });
    }
    if data.warehouse_id.trim().is_empty() {
        errors.push(ImportError {
            row,
            field: Some("warehouse_id".to_string()),
            message: "Warehouse ID is required".to_string(),
        });
    } else if data.warehouse_id.parse::<i64>().is_err() {
        errors.push(ImportError {
            row,
            field: Some("warehouse_id".to_string()),
            message: "Warehouse ID must be a valid integer".to_string(),
        });
    }
    if data.quantity.trim().is_empty() {
        errors.push(ImportError {
            row,
            field: Some("quantity".to_string()),
            message: "Quantity is required".to_string(),
        });
    } else {
        match parse_decimal(&data.quantity) {
            Ok(qty) if qty <= Decimal::ZERO => {
                errors.push(ImportError {
                    row,
                    field: Some("quantity".to_string()),
                    message: "Quantity must be positive".to_string(),
                });
            }
            Ok(_) => {}
            Err(_) => {
                errors.push(ImportError {
                    row,
                    field: Some("quantity".to_string()),
                    message: "Quantity must be a valid number".to_string(),
                });
            }
        }
    }
    if data.direction.trim().is_empty() {
        errors.push(ImportError {
            row,
            field: Some("direction".to_string()),
            message: "Direction is required".to_string(),
        });
    } else if parse_direction(&data.direction).is_err() {
        errors.push(ImportError {
            row,
            field: Some("direction".to_string()),
            message: format!(
                "Invalid direction '{}'. Valid values: Purchase, Sale, Return, Adjustment, Transfer, ProductionIn, ProductionOut, Waste, in, out",
                data.direction
            ),
        });
    }

    errors
}

/// Parse a decimal string, returning a clear error
fn parse_decimal(s: &str) -> Result<Decimal, rust_decimal::Error> {
    s.trim().parse::<Decimal>()
}

/// Simple email validation
fn is_valid_email(email: &str) -> bool {
    let email = email.trim();
    email.contains('@')
        && email
            .split('@')
            .nth(1)
            .is_some_and(|domain| domain.contains('.'))
}

/// Parse movement direction, supporting full names and shortcuts
fn parse_direction(s: &str) -> Result<MovementType, String> {
    let s = s.trim();
    if let Ok(mt) = s.parse::<MovementType>() {
        return Ok(mt);
    }
    match s.to_lowercase().as_str() {
        "in" => Ok(MovementType::Purchase),
        "out" => Ok(MovementType::Sale),
        _ => Err(format!("Invalid direction: {}", s)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_product_row() {
        let row = ProductImportRow {
            code: "P001".to_string(),
            name: "Test".to_string(),
            unit_price: "100.00".to_string(),
            category: None,
            unit: None,
        };
        let errors = validate_product_row(1, &row);
        assert!(errors.is_empty());

        let bad = ProductImportRow {
            code: "".to_string(),
            name: "".to_string(),
            unit_price: "-10".to_string(),
            category: None,
            unit: None,
        };
        let errors = validate_product_row(2, &bad);
        assert_eq!(errors.len(), 3);
    }

    #[test]
    fn test_validate_cari_row() {
        let row = CariImportRow {
            code: "C001".to_string(),
            name: "Test".to_string(),
            cari_type: "customer".to_string(),
            tax_number: None,
            email: Some("test@example.com".to_string()),
        };
        let errors = validate_cari_row(1, &row);
        assert!(errors.is_empty());

        let bad = CariImportRow {
            code: "".to_string(),
            name: "Test".to_string(),
            cari_type: "invalid".to_string(),
            tax_number: None,
            email: Some("bad-email".to_string()),
        };
        let errors = validate_cari_row(2, &bad);
        assert_eq!(errors.len(), 3);
    }

    #[test]
    fn test_validate_chart_account_row() {
        let row = ChartAccountImportRow {
            code: "100".to_string(),
            name: "Cash".to_string(),
            account_type: "Asset".to_string(),
            parent_code: None,
        };
        let errors = validate_chart_account_row(1, &row);
        assert!(errors.is_empty());
    }

    #[test]
    fn test_validate_stock_movement_row() {
        let row = StockMovementImportRow {
            product_code: "P001".to_string(),
            warehouse_id: "1".to_string(),
            quantity: "10".to_string(),
            direction: "in".to_string(),
        };
        let errors = validate_stock_movement_row(1, &row);
        assert!(errors.is_empty());

        let bad = StockMovementImportRow {
            product_code: "".to_string(),
            warehouse_id: "abc".to_string(),
            quantity: "-5".to_string(),
            direction: "up".to_string(),
        };
        let errors = validate_stock_movement_row(2, &bad);
        assert_eq!(errors.len(), 4);
    }

    #[test]
    fn test_parse_direction() {
        assert_eq!(parse_direction("in").unwrap(), MovementType::Purchase);
        assert_eq!(parse_direction("out").unwrap(), MovementType::Sale);
        assert_eq!(parse_direction("Purchase").unwrap(), MovementType::Purchase);
        assert!(parse_direction("unknown").is_err());
    }
}
