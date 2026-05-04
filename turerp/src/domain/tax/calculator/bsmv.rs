//! BSMV (Banka ve Sigorta Muameleleri Vergisi) calculator
//! — Turkish Banking and Insurance Transaction Tax
//!
//! Standard BSMV rate: 5%

use rust_decimal::Decimal;
use rust_decimal_macros::dec;

use super::TaxCalculator;
use crate::domain::tax::model::{TaxCalculationResult, TaxType};

/// BSMV calculator supporting inclusive and exclusive tax calculation
pub struct BsmvCalculator;

impl BsmvCalculator {
    /// Standard BSMV rate: 5%
    pub const RATE_STANDARD: Decimal = dec!(5);
}

impl TaxCalculator for BsmvCalculator {
    fn tax_type(&self) -> TaxType {
        TaxType::BSMV
    }

    fn calculate(
        &self,
        base_amount: Decimal,
        rate: Decimal,
        inclusive: bool,
    ) -> TaxCalculationResult {
        let tax_amount = if inclusive {
            let net = base_amount / (Decimal::ONE + rate / Decimal::new(100, 0));
            let tax = base_amount - net;
            tax.round_dp(2)
        } else {
            let tax = base_amount * rate / Decimal::new(100, 0);
            tax.round_dp(2)
        };

        TaxCalculationResult {
            base_amount,
            tax_type: TaxType::BSMV,
            rate,
            tax_amount,
            inclusive,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bsmv_exclusive_standard_rate() {
        let calc = BsmvCalculator;
        let result = calc.calculate(dec!(1000), dec!(5), false);
        assert_eq!(result.tax_amount, dec!(50));
        assert_eq!(result.base_amount, dec!(1000));
        assert_eq!(result.tax_type, TaxType::BSMV);
        assert!(!result.inclusive);
    }

    #[test]
    fn bsmv_inclusive_standard_rate() {
        let calc = BsmvCalculator;
        let result = calc.calculate(dec!(1050), dec!(5), true);
        // net = 1050 / 1.05 = 1000, tax = 50
        assert_eq!(result.tax_amount, dec!(50));
        assert_eq!(result.base_amount, dec!(1050));
        assert!(result.inclusive);
    }

    #[test]
    fn bsmv_zero_rate() {
        let calc = BsmvCalculator;
        let result = calc.calculate(dec!(1000), dec!(0), false);
        assert_eq!(result.tax_amount, dec!(0));
    }

    #[test]
    fn bsmv_exclusive_rounding() {
        let calc = BsmvCalculator;
        let result = calc.calculate(dec!(333), dec!(5), false);
        // 333 * 5 / 100 = 16.65
        assert_eq!(result.tax_amount, dec!(16.65));
    }

    #[test]
    fn bsmv_default_rate_constant() {
        assert_eq!(BsmvCalculator::RATE_STANDARD, dec!(5));
    }
}
