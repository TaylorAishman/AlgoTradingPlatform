//! R-333: the shadow simulation of a trading day starts from the paper book at
//! the previous close and fills at the raw open.

mod common;

use common::*;
use pt_shadow::daily::shadow_day;

fn within(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-12, "{a} != {b}");
}

// R-333
#[test]
fn trading_day_starts_from_the_previous_close_and_fills_at_the_raw_open() {
    let store = shadow_store();
    let cfg = config();

    let day = shadow_day(&store, &cfg, d(2024, 1, 3)).unwrap();

    assert_eq!(day.date, d(2024, 1, 3));
    // Shadow equity 100925 = 100000 + 500 + 500 - 75.
    within(day.shadow_return, 0.00925);
    // Paper equity 100960 against 100000.
    within(day.paper_return, 0.0096);
    within(day.diff, 0.00035);
}
