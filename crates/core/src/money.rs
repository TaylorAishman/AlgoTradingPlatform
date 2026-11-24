//! Decimal price and money types.

use rust_decimal::Decimal;

use crate::error::CoreError;

/// A strictly positive decimal price.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Price(Decimal);

/// A signed decimal amount of money.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Money(pub Decimal);

impl Price {
    /// Wrap a decimal, rejecting zero and negative values.
    pub fn new(d: Decimal) -> Result<Self, CoreError> {
        if d > Decimal::ZERO {
            Ok(Price(d))
        } else {
            Err(CoreError::NonPositivePrice)
        }
    }

    /// The underlying decimal.
    pub fn value(&self) -> Decimal {
        self.0
    }

    /// Exact value of a signed whole-share position at this price.
    pub fn notional(&self, shares: i64) -> Money {
        Money(self.0 * Decimal::from(shares))
    }
}

/// Round to 6 decimal places, with midpoints rounding away from zero.
pub fn round6(d: Decimal) -> Decimal {
    d.round_dp_with_strategy(6, rust_decimal::RoundingStrategy::MidpointAwayFromZero)
}
