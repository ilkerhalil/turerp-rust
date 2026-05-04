//! Damga (Damga Vergisi) calculator — Turkish Stamp Tax
//!
//! Damga vergisi is calculated per-thousand (binde):
//! rate of 9.48 per thousand (0.948%).
//! Always exclusive — stamp tax is added on top, never embedded.

use rust_decimal::Decimal;
use rust_decimal_macros::dec;

use super::TaxCalculator;
use crate::domain::tax::model::{TaxCalculationResult, TaxType};

/// Damga calculator — always exclusive, per-thousand calculation
pub struct DamgaCalculator;

impl DamgaCalculator {
    /// Standard damga rate: 9.48 per thousand (binde 9.48)
    /// Stored as the per-thousand value; actual percentage is 0.948%
    pub const RATE_PER_THOUSAND: Decimal = dec!(9.48);
}

impl TaxCalculator for DamgaCalculator {
    fn tax_type(&self) -> TaxType {
        TaxType::Damga
    }

    fn calculate(
        &self,
        base_amount: Decimal,
        rate: Decimal,
        _inclusive: bool,
    ) -> TaxCalculationResult {
        // Damga is always exclusive — stamp tax is never included in the base
        // rate here is per-thousand (binde), so: tax = base_amount * rate / 1000
        let tax_amount = (base_amount * rate / Decimal::new(1000, 0)).round_dp(2);

        TaxCalculationResult {
            base_amount,
            tax_type: TaxType::Damga,
            rate,
            tax_amount,
            inclusive: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn damga_standard_rate() {
        let calc = DamgaCalculator;
        let result = calc.calculate(dec!(10000), dec!(9.48), false);
        // 10000 * 9.48 / 1000 = 94.80
        assert_eq!(result.tax_amount, dec!(94.8));
        assert_eq!(result.base_amount, dec!(10000));
        assert_eq!(result.tax_type, TaxType::Damga);
        assert!(!result.inclusive);
    }

    #[test]
    fn damga_always_exclusive_even_when_inclusive_flag_set() {
        let calc = DamgaCalculator;
        // Damga ignores inclusive flag — always exclusive
        let result = calc.calculate(dec!(10000), dec!(9.48), true);
        assert_eq!(result.tax_amount, dec!(94.8));
        assert!(!result.inclusive);
    }

    #[test]
    fn damga_zero_base_amount() {
        let calc = DamgaCalculator;
        let result = calc.calculate(dec!(0), dec!(9.48), false);
        assert_eq!(result.tax_amount, dec!(0));
    }

    #[test]
    fn damga_rounding() {
        let calc = DamgaCalculator;
        let result = calc.calculate(dec!(3333), dec!(9.48), false);
        // 3333 * 9.48 / 1000 = 31.59684 -> rescale to 2dp
        // rust_decimal rescale(2) rounds to 31.60
        assert_eq!(result.tax_amount, dec!(31.60));
    }

    #[test]
    fn damga_custom_rate() {
        let calc = DamgaCalculator;
        // A different per-thousand rate, e.g., 5 per thousand
        let result = calc.calculate(dec!(10000), dec!(5), false);
        // 10000 * 5 / 1000 = 50
        assert_eq!(result.tax_amount, dec!(50));
    }

    #[test]
    fn damga_default_rate_constant() {
        assert_eq!(DamgaCalculator::RATE_PER_THOUSAND, dec!(9.48));
    }
}
