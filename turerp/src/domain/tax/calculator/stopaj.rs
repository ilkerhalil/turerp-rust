//! Stopaj (Gelir Vergisi Stopaji) calculator — Turkish Income Tax Withholding
//!
//! Stopaj rates: 0%, 10%, 15%, 20% depending on income type.
//! Stopaj is always calculated exclusively (never included in the base amount).

use rust_decimal::Decimal;
use rust_decimal_macros::dec;

use super::TaxCalculator;
use crate::domain::tax::model::{TaxCalculationResult, TaxType};

/// Stopaj calculator — always exclusive calculation
pub struct StopajCalculator;

impl StopajCalculator {
    /// 0% stopaj rate (exempt)
    pub const RATE_0: Decimal = dec!(0);
    /// 10% stopaj rate (e.g., rental income)
    pub const RATE_10: Decimal = dec!(10);
    /// 15% stopaj rate (e.g., freelance services)
    pub const RATE_15: Decimal = dec!(15);
    /// 20% stopaj rate (e.g., dividend withholding)
    pub const RATE_20: Decimal = dec!(20);
}

impl TaxCalculator for StopajCalculator {
    fn tax_type(&self) -> TaxType {
        TaxType::Stopaj
    }

    fn calculate(
        &self,
        base_amount: Decimal,
        rate: Decimal,
        _inclusive: bool,
    ) -> TaxCalculationResult {
        // Stopaj is always calculated exclusively regardless of the inclusive flag
        // because withholding tax is deducted from the gross amount, never embedded in it.
        let tax_amount = (base_amount * rate / Decimal::new(100, 0)).round_dp(2);

        TaxCalculationResult {
            base_amount,
            tax_type: TaxType::Stopaj,
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
    fn stopaj_exclusive_10_percent() {
        let calc = StopajCalculator;
        let result = calc.calculate(dec!(1000), dec!(10), false);
        assert_eq!(result.tax_amount, dec!(100));
        assert_eq!(result.base_amount, dec!(1000));
        assert_eq!(result.tax_type, TaxType::Stopaj);
        assert!(!result.inclusive);
    }

    #[test]
    fn stopaj_exclusive_15_percent() {
        let calc = StopajCalculator;
        let result = calc.calculate(dec!(1000), dec!(15), false);
        assert_eq!(result.tax_amount, dec!(150));
    }

    #[test]
    fn stopaj_exclusive_20_percent() {
        let calc = StopajCalculator;
        let result = calc.calculate(dec!(1000), dec!(20), false);
        assert_eq!(result.tax_amount, dec!(200));
    }

    #[test]
    fn stopaj_always_exclusive_even_when_inclusive_flag_set() {
        let calc = StopajCalculator;
        // Stopaj ignores the inclusive flag — always calculates exclusively
        let result = calc.calculate(dec!(1000), dec!(10), true);
        assert_eq!(result.tax_amount, dec!(100));
        assert!(!result.inclusive); // Always false for stopaj
    }

    #[test]
    fn stopaj_zero_rate() {
        let calc = StopajCalculator;
        let result = calc.calculate(dec!(1000), dec!(0), false);
        assert_eq!(result.tax_amount, dec!(0));
    }

    #[test]
    fn stopaj_rounding() {
        let calc = StopajCalculator;
        let result = calc.calculate(dec!(333), dec!(15), false);
        // 333 * 15 / 100 = 49.95
        assert_eq!(result.tax_amount, dec!(49.95));
    }

    #[test]
    fn stopaj_default_rate_constants() {
        assert_eq!(StopajCalculator::RATE_0, dec!(0));
        assert_eq!(StopajCalculator::RATE_10, dec!(10));
        assert_eq!(StopajCalculator::RATE_15, dec!(15));
        assert_eq!(StopajCalculator::RATE_20, dec!(20));
    }
}
