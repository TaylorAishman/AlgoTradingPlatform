//! R-508: a deferred or skipped rebalance sends nothing.

mod common;

use common::{d, sessions_2024, LiveFixture};
use pt_gate::owner::Stage;
use pt_live::cycle::{run_cycle, RunStatus};
use pt_store::trading::{DecisionRow, Mode};

fn assert_ok_and_nothing_sent(fx: &LiveFixture) {
    let outcome = run_cycle(&fx.ctx()).expect("run");

    assert_eq!(outcome.status, RunStatus::Ok, "{outcome:?}");
    assert!(!outcome.phases.iter().any(|p| p == "submit"), "{outcome:?}");
    let posts: Vec<_> = fx
        .mock
        .requests()
        .into_iter()
        .filter(|r| r.method == "POST")
        .collect();
    assert!(posts.is_empty(), "{posts:?}");
}

// R-508
#[test]
fn a_deferred_rebalance_ends_ok_without_submit_or_post() {
    let mut fx = LiveFixture::new(Stage::Paper);
    fx.cfg.risk.min_minutes_before_close = 30;
    // Open, 20 minutes before the 16:00 close.
    fx.mock.set_clock(
        "2024-03-01T15:40:00-05:00",
        true,
        "2024-03-04T09:30:00-05:00",
        "2024-03-01T16:00:00-05:00",
    );

    assert_ok_and_nothing_sent(&fx);
}

// R-508
#[test]
fn a_skipped_late_rebalance_ends_ok_without_submit_or_post() {
    let mut fx = LiveFixture::new(Stage::Paper);
    fx.cfg.risk.max_rebalance_delay_sessions = 2;
    // Bars for the four sessions after the fixture's 41, through 2024-03-06.
    let mut raw = Vec::new();
    for (i, date) in sessions_2024().iter().enumerate().take(45).skip(41) {
        let i = i as i64;
        for (name, close) in [
            ("SPY", (100 + i + (i % 2)) as f64),
            ("TLT", (1000 - 5 * i + 2 * (i % 3)) as f64 / 10.0),
            ("BIL", (10000 + i) as f64 / 100.0),
        ] {
            raw.push((
                name.to_string(),
                date.to_string(),
                close,
                close,
                close,
                close,
                1_000_000u64,
            ));
        }
    }
    fx.mock.seed_bars("raw", "sip", raw.clone());
    fx.mock.seed_bars("all", "sip", raw);
    // Pre-open on 2024-03-07, four sessions after the normal trade date 2024-03-01.
    fx.mock.set_clock(
        "2024-03-07T08:00:00-05:00",
        false,
        "2024-03-07T09:30:00-05:00",
        "2024-03-07T16:00:00-05:00",
    );
    fx.store
        .insert_decision(&DecisionRow {
            id: 0,
            strategy_id: "tsmom-v1".to_string(),
            signal_date: d(2024, 1, 31),
            trade_date: Some(d(2024, 2, 1)),
            mode: Mode::Paper,
            status: "planned".to_string(),
            inputs_json: "{}".to_string(),
            outputs_json: "{\"orders\":[]}".to_string(),
            created_at: fx.now(),
        })
        .expect("earlier decision");

    assert_ok_and_nothing_sent(&fx);
    let skipped = fx
        .store
        .decisions(Mode::Paper)
        .expect("decisions")
        .into_iter()
        .filter(|r| r.status == "skipped_late")
        .count();
    assert_eq!(skipped, 1, "the run did not reach SkippedLate");
}
