//! R-302: an equity halt is set once and announced.

mod common;

use common::LiveFixture;
use pt_gate::owner::Stage;
use pt_live::snapshot::check_halts;
use pt_risk::halts::HaltReason;
use pt_store::trading::{Book, EquitySnapshot};
use rust_decimal::Decimal;
use std::str::FromStr;

fn dec(s: &str) -> Decimal {
    Decimal::from_str(s).expect("decimal")
}

fn snapshots(fx: &LiveFixture, equities: &[(&str, &str)]) {
    for (date, equity) in equities {
        fx.store
            .put_equity_snapshot(&EquitySnapshot {
                book: Book::Paper,
                date: date.parse().expect("date"),
                cash: dec(equity),
                positions_value: Decimal::ZERO,
                equity: dec(equity),
                recorded_at: fx.now(),
            })
            .expect("snapshot");
    }
}

fn halt_set_alerts(fx: &LiveFixture) -> Vec<(String, String, String, bool)> {
    fx.store
        .alerts()
        .expect("alerts")
        .into_iter()
        .filter(|a| a.1 == "halt_set")
        .collect()
}

// R-302
#[test]
fn a_daily_loss_sets_one_halt_and_a_critical_alert() {
    let fx = LiveFixture::new(Stage::Paper);
    snapshots(&fx, &[("2024-02-29", "100000"), ("2024-03-01", "96900")]);

    let reason = check_halts(&fx.ctx()).expect("check");

    assert_eq!(reason, Some(HaltReason::DailyLoss));
    let halts = fx.store.active_halts().expect("halts");
    assert_eq!(halts.len(), 1);
    assert_eq!(halts[0].reason, "daily_loss");
    let alerts = halt_set_alerts(&fx);
    assert_eq!(alerts.len(), 1);
    assert_eq!(alerts[0].0, "critical");
}

// R-302
#[test]
fn checking_again_does_not_add_a_second_halt() {
    let fx = LiveFixture::new(Stage::Paper);
    snapshots(&fx, &[("2024-02-29", "100000"), ("2024-03-01", "96900")]);

    check_halts(&fx.ctx()).expect("first check");
    let again = check_halts(&fx.ctx()).expect("second check");

    assert_eq!(again, Some(HaltReason::DailyLoss));
    assert_eq!(fx.store.halts().expect("halts").len(), 1);
    assert_eq!(fx.store.active_halts().expect("halts").len(), 1);
    assert_eq!(halt_set_alerts(&fx).len(), 1);
}

// R-302
#[test]
fn a_small_loss_returns_none_and_sets_nothing() {
    let fx = LiveFixture::new(Stage::Paper);
    snapshots(&fx, &[("2024-02-29", "100000"), ("2024-03-01", "99000")]);

    let reason = check_halts(&fx.ctx()).expect("check");

    assert_eq!(reason, None);
    assert!(fx.store.halts().expect("halts").is_empty());
    assert!(halt_set_alerts(&fx).is_empty());
}

// R-302
#[test]
fn fewer_than_two_snapshots_never_halt() {
    let fx = LiveFixture::new(Stage::Paper);
    snapshots(&fx, &[("2024-03-01", "50000")]);

    assert_eq!(check_halts(&fx.ctx()).expect("check"), None);
    assert!(fx.store.halts().expect("halts").is_empty());
}
