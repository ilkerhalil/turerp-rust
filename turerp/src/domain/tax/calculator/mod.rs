//! Tax calculator modules for each Turkish tax type
//!
//! Provides a `TaxCalculator` trait and concrete implementations for:
//! - KDV (Value Added Tax): inclusive/exclusive, rates 1%/10%/20%
//! - OIV (Special Communication Tax): inclusive/exclusive, rate 7.5%
//! - BSMV (Banking and Insurance Transaction Tax): inclusive/exclusive, rate 5%
//! - Damga (Stamp Tax): always exclusive, 9.48 per thousand
//! - Stopaj (Income Tax Withholding): always exclusive, rates 0%/10%/15%/20%

use crate::domain::tax::model::{TaxCalculationResult, TaxType};
use rust_decimal::Decimal;

pub mod bsmv;
pub mod damga;
pub mod kdv;
pub mod oiv;
pub mod stopaj;

pub use bsmv::BsmvCalculator;
pub use damga::DamgaCalculator;
pub use kdv::KdvCalculator;
pub use oiv::OivCalculator;
pub use stopaj::StopajCalculator;

/// Trait for tax calculation strategies.
///
/// Each Turkish tax type implements this trait to provide type-specific
/// calculation logic. Some taxes (KDV, OIV, BSMV) support inclusive
/// calculation where the base amount already contains the tax, while
/// others (Damga, Stopaj) are always exclusive.
pub trait TaxCalculator: Send + Sync {
    /// Returns the tax type this calculator handles
    fn tax_type(&self) -> TaxType;

    /// Calculate tax for the given base amount and rate.
    ///
    /// # Arguments
    /// * `base_amount` — The monetary amount to calculate tax on
    /// * `rate` — The tax rate (percentage for KDV/OIV/BSMV/Stopaj, per-thousand for Damga)
    /// * `inclusive` — Whether the base amount already includes the tax.
    ///   Note: Damga and Stopaj always calculate exclusively regardless of this flag.
    fn calculate(
        &self,
        base_amount: Decimal,
        rate: Decimal,
        inclusive: bool,
    ) -> TaxCalculationResult;
}

/// Factory function returning the appropriate calculator for a tax type.
///
/// KV and GV use `StopajCalculator` as their calculation logic is identical
/// (always exclusive percentage-based).
pub fn get_calculator(tax_type: TaxType) -> Box<dyn TaxCalculator> {
    match tax_type {
        TaxType::KDV => Box::new(KdvCalculator),
        TaxType::OIV => Box::new(OivCalculator),
        TaxType::BSMV => Box::new(BsmvCalculator),
        TaxType::Damga => Box::new(DamgaCalculator),
        TaxType::Stopaj => Box::new(StopajCalculator),
        TaxType::KV => Box::new(StopajCalculator),
        TaxType::GV => Box::new(StopajCalculator),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn get_calculator_returns_correct_type() {
        assert_eq!(get_calculator(TaxType::KDV).tax_type(), TaxType::KDV);
        assert_eq!(get_calculator(TaxType::OIV).tax_type(), TaxType::OIV);
        assert_eq!(get_calculator(TaxType::BSMV).tax_type(), TaxType::BSMV);
        assert_eq!(get_calculator(TaxType::Damga).tax_type(), TaxType::Damga);
        assert_eq!(get_calculator(TaxType::Stopaj).tax_type(), TaxType::Stopaj);
        // KV and GV use StopajCalculator
        assert_eq!(get_calculator(TaxType::KV).tax_type(), TaxType::Stopaj);
        assert_eq!(get_calculator(TaxType::GV).tax_type(), TaxType::Stopaj);
    }

    #[test]
    fn factory_kdv_calculation() {
        let calc = get_calculator(TaxType::KDV);
        let result = calc.calculate(dec!(1000), dec!(20), false);
        assert_eq!(result.tax_amount, dec!(200));
        assert_eq!(result.tax_type, TaxType::KDV);
    }

    #[test]
    fn factory_damga_calculation() {
        let calc = get_calculator(TaxType::Damga);
        let result = calc.calculate(dec!(10000), dec!(9.48), false);
        assert_eq!(result.tax_amount, dec!(94.8));
        assert_eq!(result.tax_type, TaxType::Damga);
        assert!(!result.inclusive);
    }
}
