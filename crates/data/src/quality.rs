//! Data quality checks on bars.

use crate::error::DataError;
use pt_alpaca::data::WireBar;
use rust_decimal::Decimal;

/// R-124: a bar must be internally consistent. All prices are positive, the
/// high is at least every other price and the low is at most every other price.
pub fn check_bar(bar: &WireBar) -> Result<(), DataError> {
    let bad = |message: &str| DataError::BadBar {
        symbol: bar.symbol.clone(),
        date: bar.date,
        message: message.to_string(),
    };
    let prices = [bar.open, bar.high, bar.low, bar.close];
    if prices.iter().any(|p| *p <= Decimal::ZERO) {
        return Err(bad("prices must be positive"));
    }
    if prices.iter().any(|p| *p > bar.high) {
        return Err(bad("high is below another price"));
    }
    if prices.iter().any(|p| *p < bar.low) {
        return Err(bad("low is above another price"));
    }
    Ok(())
}
