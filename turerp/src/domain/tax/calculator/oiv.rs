//! OIV (Ozel Iletisim Vergisi) calculator — Turkish Special Communication Tax
//!
//! Standard OIV rate: 7.5%

use rust_decimal::Decimal;
use rust_decimal_macros::dec;

use super::TaxCalculator;
use crate::domain::tax::model::{TaxCalculationResult, TaxType};

/// OIV calculator supporting inclusive and exclusive tax calculation
pub struct OivCalculator;

impl OivCalculator {
    /// Standard OIV rate: 7.5%
    pub const RATE_STANDARD: Decimal = dec!(7.5);
}

impl TaxCalculator for OivCalculator {
    fn tax_type(&self) -> TaxType {
        TaxType::OIV
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
            tax_type: TaxType::OIV,
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
    fn oiv_exclusive_standard_rate() {
        let calc = OivCalculator;
        let result = calc.calculate(dec!(1000), dec!(7.5), false);
        assert_eq!(result.tax_amount, dec!(75));
        assert_eq!(result.base_amount, dec!(1000));
        assert_eq!(result.tax_type, TaxType::OIV);
        assert!(!result.inclusive);
    }

    #[test]
    fn oiv_inclusive_standard_rate() {
        let calc = OivCalculator;
        let result = calc.calculate(dec!(1075), dec!(7.5), true);
        // net = 1075 / 1.075 = 1000, tax = 75
        assert_eq!(result.tax_amount, dec!(75));
        assert_eq!(result.base_amount, dec!(1075));
        assert!(result.inclusive);
    }

    #[test]
    fn oiv_zero_rate() {
        let calc = OivCalculator;
        let result = calc.calculate(dec!(1000), dec!(0), false);
        assert_eq!(result.tax_amount, dec!(0));
    }

    #[test]
    fn oiv_exclusive_rounding() {
        let calc = OivCalculator;
        let result = calc.calculate(dec!(333), dec!(7.5), false);
        // 333 * 7.5 / 100 = 24.975 -> rescale to 2dp = 24.98 (banker's rounding)
        assert_eq!(result.tax_amount, dec!(24.98));
    }

    #[test]
    fn oiv_default_rate_constant() {
        assert_eq!(OivCalculator::RATE_STANDARD, dec!(7.5));
    }
}
