//! Equity-based trading halts.

use crate::pretrade::RiskLimits;
use rust_decimal::Decimal;

/// Why trading should halt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HaltReason {
    /// The latest day-over-day loss reached the daily loss limit.
    DailyLoss,
    /// Equity fell from its running peak by at least the drawdown limit.
    Drawdown,
}

/// Check an equity series (oldest first) against the halt limits.
///
/// Halts with `DailyLoss` when the latest day-over-day loss, as a fraction of
/// the previous equity, reaches `daily_loss_limit`. Fails closed: if the previous
/// or the latest equity is not positive it halts with `DailyLoss` too. Otherwise
/// halts with `Drawdown` when the latest equity is at least `max_drawdown` below
/// the running peak. Fewer than two points never halt.
pub fn check_equity(equity: &[Decimal], limits: &RiskLimits) -> Option<HaltReason> {
    let [.., prev, last] = equity else {
        return None;
    };
    if *prev <= Decimal::ZERO || *last <= Decimal::ZERO {
        // Fail closed: equity that is not positive cannot be measured.
        return Some(HaltReason::DailyLoss);
    }
    let loss = *prev - *last;
    if loss > Decimal::ZERO && loss >= limits.daily_loss_limit * *prev {
        return Some(HaltReason::DailyLoss);
    }
    let peak = equity.iter().copied().max()?;
    let drop = peak - *last;
    if drop > Decimal::ZERO && drop >= limits.max_drawdown * peak {
        return Some(HaltReason::Drawdown);
    }
    None
}
