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

// R-185
#[test]
fn loss_exactly_at_the_limit_is_a_daily_loss_halt() {
    assert_eq!(
        check_equity(&series(&[100_000, 97_000]), &limits()),
        Some(HaltReason::DailyLoss)
    );
}

// R-185
#[test]
fn loss_just_inside_the_limit_is_not_a_halt() {
    assert_eq!(check_equity(&series(&[100_000, 97_001]), &limits()), None);
}

// R-185
#[test]
fn loss_beyond_the_limit_is_a_daily_loss_halt() {
    assert_eq!(
        check_equity(&series(&[100_000, 95_000]), &limits()),
        Some(HaltReason::DailyLoss)
    );
}

// R-185
#[test]
fn single_element_series_is_not_a_halt() {
    assert_eq!(check_equity(&series(&[100_000]), &limits()), None);
}

// R-185
#[test]
fn empty_series_is_not_a_halt() {
    assert_eq!(check_equity(&[], &limits()), None);
}

// R-185
#[test]
fn a_gain_is_not_a_halt() {
    assert_eq!(check_equity(&series(&[100_000, 110_000]), &limits()), None);
}

// R-185: only the latest day counts; an old large one-day loss that has recovered is not a daily-loss halt.
#[test]
fn only_the_latest_day_is_tested_for_daily_loss() {
    assert_eq!(
        check_equity(&series(&[100_000, 96_000, 100_000, 100_500]), &limits()),
        None
    );
}

// R-185
#[test]
fn latest_day_loss_after_earlier_history_is_a_halt() {
    assert_eq!(
        check_equity(&series(&[100_000, 101_000, 97_900, 94_900]), &limits()),
        Some(HaltReason::DailyLoss)
    );
}

// R-185: fail closed on equity that is not positive.
#[test]
fn latest_equity_of_zero_is_a_daily_loss_halt() {
    assert_eq!(
        check_equity(&series(&[100_000, 0]), &limits()),
        Some(HaltReason::DailyLoss)
    );
}

// R-185
#[test]
fn negative_latest_equity_is_a_daily_loss_halt() {
    assert_eq!(
        check_equity(&series(&[100_000, -500]), &limits()),
        Some(HaltReason::DailyLoss)
    );
}

// R-185
#[test]
fn non_positive_previous_equity_is_a_daily_loss_halt() {
    assert_eq!(
        check_equity(&series(&[0, 100_000]), &limits()),
        Some(HaltReason::DailyLoss)
    );
    assert_eq!(
        check_equity(&series(&[-1, 100_000]), &limits()),
        Some(HaltReason::DailyLoss)
    );
}

// R-185: a single point is None whatever its value.
#[test]
fn single_zero_element_series_is_not_a_halt() {
    assert_eq!(check_equity(&series(&[0]), &limits()), None);
}
