//! KDV (Katma Deger Vergisi) calculator — Turkish Value Added Tax
//!
//! Standard KDV rates: 1%, 10%, 20%

use rust_decimal::Decimal;
use rust_decimal_macros::dec;

use super::TaxCalculator;
use crate::domain::tax::model::{TaxCalculationResult, TaxType};

/// KDV calculator supporting inclusive and exclusive tax calculation
pub struct KdvCalculator;

/// Default KDV rate brackets
impl KdvCalculator {
    /// 1% KDV rate (reduced rate for basic necessities)
    pub const RATE_1: Decimal = dec!(1);
    /// 10% KDV rate (intermediate rate)
    pub const RATE_10: Decimal = dec!(10);
    /// 20% KDV rate (standard rate)
    pub const RATE_20: Decimal = dec!(20);
}

impl TaxCalculator for KdvCalculator {
    fn tax_type(&self) -> TaxType {
        TaxType::KDV
    }

    fn calculate(
        &self,
        base_amount: Decimal,
        rate: Decimal,
        inclusive: bool,
    ) -> TaxCalculationResult {
        let tax_amount = if inclusive {
            // Inclusive: base_amount includes tax.
            // net = base_amount / (1 + rate/100)
            // tax = base_amount - net
            let net = base_amount / (Decimal::ONE + rate / Decimal::new(100, 0));
            let tax = base_amount - net;
            tax.round_dp(2)
        } else {
            // Exclusive: tax is added on top
            let tax = base_amount * rate / Decimal::new(100, 0);
            tax.round_dp(2)
        };

        TaxCalculationResult {
            base_amount,
            tax_type: TaxType::KDV,
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
    fn kdv_exclusive_20_percent() {
        let calc = KdvCalculator;
        let result = calc.calculate(dec!(1000), dec!(20), false);
        assert_eq!(result.tax_amount, dec!(200));
        assert_eq!(result.base_amount, dec!(1000));
        assert_eq!(result.tax_type, TaxType::KDV);
        assert!(!result.inclusive);
    }

    #[test]
    fn kdv_inclusive_20_percent() {
        let calc = KdvCalculator;
        let result = calc.calculate(dec!(1200), dec!(20), true);
        // net = 1200 / 1.2 = 1000, tax = 200
        assert_eq!(result.tax_amount, dec!(200));
        assert_eq!(result.base_amount, dec!(1200));
        assert!(result.inclusive);
    }

    #[test]
    fn kdv_exclusive_10_percent() {
        let calc = KdvCalculator;
        let result = calc.calculate(dec!(1000), dec!(10), false);
        assert_eq!(result.tax_amount, dec!(100));
    }

    #[test]
    fn kdv_exclusive_1_percent() {
        let calc = KdvCalculator;
        let result = calc.calculate(dec!(1000), dec!(1), false);
        assert_eq!(result.tax_amount, dec!(10));
    }

    #[test]
    fn kdv_zero_rate() {
        let calc = KdvCalculator;
        let result = calc.calculate(dec!(1000), dec!(0), false);
        assert_eq!(result.tax_amount, dec!(0));
    }

    #[test]
    fn kdv_inclusive_zero_rate() {
        let calc = KdvCalculator;
        let result = calc.calculate(dec!(1000), dec!(0), true);
        assert_eq!(result.tax_amount, dec!(0));
    }

    #[test]
    fn kdv_exclusive_rounding() {
        let calc = KdvCalculator;
        let result = calc.calculate(dec!(333), dec!(20), false);
        // 333 * 20 / 100 = 66.6 -> rescale to 2 decimal places
        assert_eq!(result.tax_amount, dec!(66.6));
    }

    #[test]
    fn kdv_inclusive_rounding() {
        let calc = KdvCalculator;
        let result = calc.calculate(dec!(333), dec!(20), true);
        // net = 333 / 1.2 = 277.5, tax = 333 - 277.5 = 55.5
        assert_eq!(result.tax_amount, dec!(55.5));
    }

    #[test]
    fn kdv_default_rate_constants() {
        assert_eq!(KdvCalculator::RATE_1, dec!(1));
        assert_eq!(KdvCalculator::RATE_10, dec!(10));
        assert_eq!(KdvCalculator::RATE_20, dec!(20));
    }
}
