//! Pure SGK payroll calculator — no async, no I/O

use rust_decimal::Decimal;
use rust_decimal_macros::dec;

use crate::domain::hr::sgk::model::{IncomeTaxBracket, SgkConfig, SgkPayrollLineItem};

/// Turkish SGK payroll calculator (2026 rules)
pub struct PayrollCalculator {
    config: SgkConfig,
    brackets: Vec<IncomeTaxBracket>,
}

impl PayrollCalculator {
    pub fn new(config: SgkConfig, brackets: Vec<IncomeTaxBracket>) -> Self {
        Self { config, brackets }
    }

    /// Calculate net from gross salary
    pub fn gross_to_net(
        &self,
        gross: Decimal,
        bonuses: Decimal,
        marital_status: &str,
        children_count: i32,
        spouse_working: bool,
    ) -> SgkPayrollLineItem {
        let sgk_base = self.sgk_earnings_base(gross);
        let sgk_premium = sgk_base * self.config.sgk_worker_rate;
        let unemployment_premium = sgk_base * self.config.unemployment_worker_rate;
        let stamp_tax = gross * self.config.stamp_tax_rate;
        let tax_base = gross - sgk_premium - unemployment_premium;
        let income_tax = self.calculate_income_tax(tax_base);
        let agi = self.calculate_agi(marital_status, children_count, spouse_working);
        let net_salary =
            gross + bonuses - sgk_premium - unemployment_premium - income_tax - stamp_tax + agi;
        let employer_cost = gross + (sgk_base * dec!(0.225));

        SgkPayrollLineItem {
            employee_id: 0,
            gross_salary: gross,
            sgk_earnings_base: sgk_base,
            sgk_premium_worker: sgk_premium,
            unemployment_premium_worker: unemployment_premium,
            income_tax_base: tax_base,
            income_tax,
            stamp_tax,
            agi,
            net_salary,
            employer_cost,
        }
    }

    /// Calculate gross from desired net (binary search, converge within 0.01 TL)
    pub fn net_to_gross(
        &self,
        desired_net: Decimal,
        bonuses: Decimal,
        marital_status: &str,
        children_count: i32,
        spouse_working: bool,
    ) -> SgkPayrollLineItem {
        let mut low = desired_net;
        let mut high = desired_net * dec!(2.5);
        let tolerance = Decimal::new(1, 2); // 0.01
        let mut best_guess = low;

        for _ in 0..50 {
            let mid = (low + high) / dec!(2);
            let result =
                self.gross_to_net(mid, bonuses, marital_status, children_count, spouse_working);
            let diff = result.net_salary - desired_net;

            if diff.abs() < tolerance {
                best_guess = mid;
                break;
            }

            if diff > Decimal::ZERO {
                // Net too high — reduce gross
                high = mid;
            } else {
                // Net too low — increase gross
                low = mid;
            }
            best_guess = mid;
        }

        self.gross_to_net(
            best_guess,
            bonuses,
            marital_status,
            children_count,
            spouse_working,
        )
    }

    fn calculate_agi(
        &self,
        marital_status: &str,
        children_count: i32,
        spouse_working: bool,
    ) -> Decimal {
        let base = if marital_status.eq_ignore_ascii_case("married") && !spouse_working {
            self.config.agi_amount_married
        } else {
            self.config.agi_amount_single
        };
        let child_addon = Decimal::from(children_count.max(0)) * self.config.agi_per_child;
        base + child_addon
    }

    fn calculate_income_tax(&self, tax_base: Decimal) -> Decimal {
        let mut remaining = tax_base;
        let mut tax = Decimal::ZERO;

        for bracket in &self.brackets {
            if remaining <= Decimal::ZERO {
                break;
            }
            let bracket_size = match bracket.upper_limit {
                Some(upper) => upper - bracket.lower_limit,
                None => remaining, // top bracket — no upper limit
            };
            let taxable_in_bracket = remaining.min(bracket_size.max(Decimal::ZERO));
            tax += taxable_in_bracket * bracket.rate;
            remaining -= taxable_in_bracket;
        }

        tax
    }

    fn sgk_earnings_base(&self, gross: Decimal) -> Decimal {
        gross
            .min(self.config.sgk_earnings_ceiling)
            .max(self.config.min_wage)
    }
}

/// Default SGK configuration for 2026 (hardcoded)
pub fn default_sgk_config_2026() -> SgkConfig {
    SgkConfig {
        id: 0,
        tenant_id: 0,
        year: 2026,
        min_wage: dec!(28000.00),
        sgk_earnings_ceiling: dec!(150000.00),
        sgk_worker_rate: dec!(0.14),
        unemployment_worker_rate: dec!(0.01),
        stamp_tax_rate: dec!(0.00759),
        agi_amount_single: dec!(2800.00),
        agi_amount_married: dec!(3360.00),
        agi_per_child: dec!(560.00),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    }
}

/// Default income tax brackets for 2026 (hardcoded)
pub fn default_income_tax_brackets_2026() -> Vec<IncomeTaxBracket> {
    vec![
        IncomeTaxBracket {
            id: 0,
            year: 2026,
            bracket_no: 1,
            lower_limit: dec!(0),
            upper_limit: Some(dec!(158000)),
            rate: dec!(0.15),
        },
        IncomeTaxBracket {
            id: 0,
            year: 2026,
            bracket_no: 2,
            lower_limit: dec!(158000),
            upper_limit: Some(dec!(330000)),
            rate: dec!(0.20),
        },
        IncomeTaxBracket {
            id: 0,
            year: 2026,
            bracket_no: 3,
            lower_limit: dec!(330000),
            upper_limit: Some(dec!(800000)),
            rate: dec!(0.27),
        },
        IncomeTaxBracket {
            id: 0,
            year: 2026,
            bracket_no: 4,
            lower_limit: dec!(800000),
            upper_limit: Some(dec!(1900000)),
            rate: dec!(0.35),
        },
        IncomeTaxBracket {
            id: 0,
            year: 2026,
            bracket_no: 5,
            lower_limit: dec!(1900000),
            upper_limit: None,
            rate: dec!(0.40),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn calc() -> PayrollCalculator {
        PayrollCalculator::new(
            default_sgk_config_2026(),
            default_income_tax_brackets_2026(),
        )
    }

    #[test]
    fn test_gross_to_net_50k_single() {
        let c = calc();
        let result = c.gross_to_net(dec!(50000), Decimal::ZERO, "single", 0, true);

        // SGK base = 50,000
        assert_eq!(result.sgk_earnings_base, dec!(50000));
        // SGK premium = 50,000 * 0.14 = 7,000
        assert_eq!(result.sgk_premium_worker, dec!(7000));
        // Unemployment = 50,000 * 0.01 = 500
        assert_eq!(result.unemployment_premium_worker, dec!(500));
        // Stamp tax = 50,000 * 0.00759 = 379.50
        assert_eq!(result.stamp_tax, dec!(379.50));
        // Tax base = 50,000 - 7,000 - 500 = 42,500
        assert_eq!(result.income_tax_base, dec!(42500));
        // Income tax = 42,500 * 0.15 = 6,375
        assert_eq!(result.income_tax, dec!(6375));
        // AGI (single) = 2,800
        assert_eq!(result.agi, dec!(2800));
        // Net = 50,000 - 7,000 - 500 - 6,375 - 379.50 + 2,800 = 38,545.50
        assert_eq!(result.net_salary, dec!(38545.50));
    }

    #[test]
    fn test_gross_to_net_200k_hits_ceiling() {
        let c = calc();
        let result = c.gross_to_net(dec!(200000), Decimal::ZERO, "single", 0, true);

        // SGK base clamped to ceiling 150,000
        assert_eq!(result.sgk_earnings_base, dec!(150000));
        // SGK premium = 150,000 * 0.14 = 21,000
        assert_eq!(result.sgk_premium_worker, dec!(21000));
        // Unemployment = 150,000 * 0.01 = 1,500
        assert_eq!(result.unemployment_premium_worker, dec!(1500));
        // Stamp tax = 200,000 * 0.00759 = 1,518
        assert_eq!(result.stamp_tax, dec!(1518));
        // Tax base = 200,000 - 21,000 - 1,500 = 177,500
        assert_eq!(result.income_tax_base, dec!(177500));
        // Income tax:
        //   158,000 * 0.15 = 23,700
        //   19,500 * 0.20  = 3,900
        //   total = 27,600
        assert_eq!(result.income_tax, dec!(27600));
        // Net = 200,000 - 21,000 - 1,500 - 27,600 - 1,518 + 2,800 = 151,182
        assert_eq!(result.net_salary, dec!(151182));
    }

    #[test]
    fn test_net_to_gross_round_trip() {
        let c = calc();
        let original_gross = dec!(75000);
        let result = c.gross_to_net(original_gross, Decimal::ZERO, "single", 0, true);
        let desired_net = result.net_salary;

        let reverse = c.net_to_gross(desired_net, Decimal::ZERO, "single", 0, true);
        let diff = (reverse.gross_salary - original_gross).abs();
        assert!(
            diff < dec!(0.50),
            "Round-trip diff {} should be < 0.50",
            diff
        );
    }

    #[test]
    fn test_agi_married_two_children() {
        let c = calc();
        let result = c.gross_to_net(dec!(50000), Decimal::ZERO, "married", 2, false);

        // Married, spouse not working, 2 children:
        // 3,360 + 2 * 560 = 4,480
        assert_eq!(result.agi, dec!(4480));
    }

    #[test]
    fn test_progressive_income_tax_400k() {
        let c = calc();
        let result = c.gross_to_net(dec!(400000), Decimal::ZERO, "single", 0, true);

        // Tax base = 400,000 - sgk(150k*0.14=21k) - unemployment(150k*0.01=1.5k) = 377,500
        assert_eq!(result.income_tax_base, dec!(377500));
        // Income tax:
        //   158,000 * 0.15 = 23,700
        //   172,000 * 0.20 = 34,400
        //   47,500 * 0.27 = 12,825
        //   total = 70,925
        assert_eq!(result.income_tax, dec!(70925));
    }
}
