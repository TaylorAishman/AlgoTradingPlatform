use pt_risk::halts::{check_equity, HaltReason};
use pt_risk::pretrade::RiskLimits;
use rust_decimal::Decimal;

fn limits() -> RiskLimits {
    RiskLimits {
        max_order_notional: Decimal::new(1_000_000, 0),
        max_weight: Decimal::new(1, 0),
        max_gross: Decimal::new(1, 0),
        daily_loss_limit: Decimal::new(3, 2),
        max_drawdown: Decimal::new(15, 2),
    }
}

fn series(values: &[i64]) -> Vec<Decimal> {
    values.iter().map(|v| Decimal::new(*v, 0)).collect()
}

const STEADY_DECLINE: [i64; 10] = [
    100_000, 110_000, 108_000, 106_000, 104_000, 102_000, 100_000, 98_000, 96_000, 94_000,
];

fn with_last(last: i64) -> Vec<Decimal> {
    let mut values = STEADY_DECLINE.to_vec();
    values.push(last);
    series(&values)
}

// R-186: 93500 is exactly 15% below the 110000 peak, and no single day loses 3%.
#[test]
fn drawdown_exactly_at_the_limit_is_a_drawdown_halt() {
    assert_eq!(
        check_equity(&with_last(93_500), &limits()),
        Some(HaltReason::Drawdown)
    );
}

// R-186
#[test]
fn drawdown_just_inside_the_limit_is_not_a_halt() {
    assert_eq!(check_equity(&with_last(93_501), &limits()), None);
}

// R-186
// Last day: 94000 -> 91500 loses 2500, which is 2.66% of 94000 (under the 3% limit).
// Drawdown: 110000 -> 91500 loses 18500, which is 16.8% of the peak (over 15%).
#[test]
fn drawdown_beyond_the_limit_is_a_drawdown_halt() {
    assert_eq!(
        check_equity(&with_last(91_500), &limits()),
        Some(HaltReason::Drawdown)
    );
}

// R-186: when both conditions hold, DailyLoss is returned.
#[test]
fn daily_loss_takes_precedence_over_drawdown() {
    assert_eq!(
        check_equity(&series(&[100_000, 110_000, 93_000]), &limits()),
        Some(HaltReason::DailyLoss)
    );
}

// R-186: a rising series has no drawdown.
#[test]
fn rising_equity_is_not_a_halt() {
    assert_eq!(
        check_equity(&series(&[100_000, 105_000, 110_000]), &limits()),
        None
    );
}

// R-186: non-positive latest equity is a DailyLoss halt (J05 fails closed),
// and DailyLoss keeps precedence over Drawdown.
#[test]
fn non_positive_latest_equity_is_a_daily_loss_halt_not_a_drawdown() {
    assert_eq!(
        check_equity(&series(&[100_000, 110_000, 0]), &limits()),
        Some(HaltReason::DailyLoss)
    );
    assert_eq!(
        check_equity(&series(&[100_000, 110_000, -1]), &limits()),
        Some(HaltReason::DailyLoss)
    );
}

// R-186: a non-positive value earlier in the history, since recovered, is not a halt.
#[test]
fn recovered_non_positive_history_is_not_a_halt() {
    assert_eq!(
        check_equity(&series(&[0, 100_000, 100_000]), &limits()),
        None
    );
}

// R-186: a single point has no drawdown, whatever its value.
#[test]
fn single_point_series_is_not_a_drawdown() {
    assert_eq!(check_equity(&series(&[0]), &limits()), None);
    assert_eq!(check_equity(&series(&[100_000]), &limits()), None);
}
