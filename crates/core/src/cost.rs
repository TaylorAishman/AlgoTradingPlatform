//! Transaction cost parameters.

use crate::error::CoreError;
use rust_decimal::Decimal;

/// Cost model parameters. All fields must be non-negative.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CostSpec {
    pub spread_bps: Decimal,
    pub fee_bps: Decimal,
    pub impact_coeff: Decimal,
    pub borrow_bps_annual: Decimal,
}

impl CostSpec {
    /// A spec with every cost set to zero.
    pub fn zero() -> CostSpec {
        CostSpec {
            spread_bps: Decimal::ZERO,
            fee_bps: Decimal::ZERO,
            impact_coeff: Decimal::ZERO,
            borrow_bps_annual: Decimal::ZERO,
        }
    }

    /// Reject any negative field.
    pub fn validate(&self) -> Result<(), CoreError> {
        let fields = [
            ("spread_bps", self.spread_bps),
            ("fee_bps", self.fee_bps),
            ("impact_coeff", self.impact_coeff),
            ("borrow_bps_annual", self.borrow_bps_annual),
        ];
        for (name, v) in fields {
            if v.is_sign_negative() && !v.is_zero() {
                return Err(CoreError::InvalidCost(format!("{name} must be >= 0")));
            }
        }
        Ok(())
    }
}
