//! R-511, R-512, R-513: the bootstrap rule, late data and the rebalance warnings.

mod common;

use chrono::{DateTime, NaiveDate, Utc};
use common::{d, sessions_2024, sym, LiveFixture};
use pt_alpaca::reference::WireClock;
use pt_core::money::Price;
use pt_core::types::{Adjustment, Bar};
use pt_data::reference::sync_assets;
use pt_engine::decision::record_from_json;
use pt_gate::owner::Stage;
use pt_live::rebalance::{rebalance, RebalanceOutcome};
use pt_live::reconcile::reconcile;
use pt_store::market::BarRow;
use pt_store::trading::{Book, DecisionRow, EquitySnapshot, Mode};
use rust_decimal::Decimal;

fn price(x: f64) -> Price {
    Price::from_f64_shortest(x).expect("price")
}

/// The fixture's bar rows for the first 45 sessions (through 2024-03-06).
fn bar_rows() -> Vec<(String, NaiveDate, BarRow)> {
    let sessions = sessions_2024();
    let mut rows = Vec::new();
    let mut prev: [Option<f64>; 3] = [None; 3];
    for (i, date) in sessions.iter().take(45).enumerate() {
        let i = i as i64;
        let closes = [
            (100 + i + (i % 2)) as f64,
            (1000 - 5 * i + 2 * (i % 3)) as f64 / 10.0,
            (10000 + i) as f64 / 100.0,
        ];
        for (k, name) in ["SPY", "TLT", "BIL"].iter().enumerate() {
            let close = closes[k];
            let open = prev[k].unwrap_or(close);
            prev[k] = Some(close);
            for adjustment in [Adjustment::Raw, Adjustment::All] {
                rows.push((
                    name.to_string(),
                    *date,
                    BarRow {
                        bar: Bar {
                            symbol: sym(name),
                            date: *date,
                            open: price(open),
                            high: price(open.max(close)),
                            low: price(open.min(close)),
                            close: price(close),
                            volume: 1_000_000,
                        },
                        adjustment,
                        feed: "sip".to_string(),
                    },
                ));
            }
        }
    }
    rows
}

fn is_missing_bar(name: &str, date: NaiveDate) -> bool {
    name == "TLT" && date == d(2024, 3, 6)
}

fn pre_open(date: &str) -> WireClock {
    let at = |t: &str| DateTime::parse_from_rfc3339(&format!("{date}T{t}-05:00")).expect("time");
    WireClock {
        timestamp: at("08:00:00"),
        is_open: false,
        next_open: at("09:30:00"),
        next_close: at("16:00:00"),
    }
}

fn decision(mode: Mode, status: &str, fx: &LiveFixture) -> DecisionRow {
    DecisionRow {
        id: 0,
        strategy_id: "tsmom-v1".to_string(),
        signal_date: d(2024, 1, 31),
        trade_date: Some(d(2024, 2, 1)),
        mode,
        status: status.to_string(),
        inputs_json: "{}".to_string(),
        outputs_json: "{\"orders\":[]}".to_string(),
        created_at: fx.now(),
    }
}

/// The late setup: paper, bars and snapshots through 2024-03-06, limit of two sessions.
/// `earlier` is an optional earlier decision (mode, status); `tlt_bar` stores TLT's 2024-03-06 (reference date) bar;
/// `tlt_tradable` is the stored TLT snapshot flag.
fn fixture(earlier: Option<(Mode, &str)>, tlt_bar: bool, tlt_tradable: bool) -> LiveFixture {
    let mut fx = LiveFixture::new(Stage::Paper);
    fx.mock.seed_asset("TLT", tlt_tradable, true, true);
    assert!(reconcile(&fx.ctx()).expect("reconcile").clean);
    let rows: Vec<BarRow> = bar_rows()
        .into_iter()
        .filter(|(n, date, _)| tlt_bar || !is_missing_bar(n, *date))
        .map(|(_, _, r)| r)
        .collect();
    fx.store.upsert_bars(&rows, Utc::now()).expect("bars");
    sync_assets(&fx.store, &fx.http, &fx.cfg, fx.now()).expect("assets");
    for (m, day) in [(2, 29), (3, 1), (3, 4), (3, 5), (3, 6)] {
        fx.store
            .put_equity_snapshot(&EquitySnapshot {
                book: Book::Paper,
                date: d(2024, m, day),
                cash: Decimal::from(100000),
                positions_value: Decimal::ZERO,
                equity: Decimal::from(100000),
                recorded_at: fx.now(),
            })
            .expect("snapshot");
    }
    if let Some((mode, status)) = earlier {
        fx.store
            .insert_decision(&decision(mode, status, &fx))
            .expect("earlier decision");
    }
    fx.cfg.risk.max_rebalance_delay_sessions = 2;
    fx
}

fn run(fx: &LiveFixture) -> RebalanceOutcome {
    rebalance(&fx.ctx(), &pre_open("2024-03-07")).expect("rebalance")
}

fn alert_levels(fx: &LiveFixture, code: &str) -> Vec<String> {
    fx.store
        .alerts()
        .expect("alerts")
        .into_iter()
        .filter(|a| a.1 == code)
        .map(|a| a.0)
        .collect()
}

// R-511
#[test]
fn with_no_earlier_paper_decision_a_late_rebalance_is_recorded_as_bootstrap() {
    let fx = fixture(None, true, true);
    let outcome = run(&fx);
    assert!(
        matches!(outcome, RebalanceOutcome::Recorded { .. }),
        "got {outcome:?}"
    );
    let row = fx
        .store
        .decision_for("tsmom-v1", d(2024, 2, 29), Mode::Paper)
        .expect("lookup")
        .expect("row");
    assert!(
        record_from_json(&row.inputs_json)
            .expect("record")
            .bootstrap
    );
}

// R-511
#[test]
fn an_earlier_skipped_late_paper_decision_counts_so_a_late_rebalance_is_skipped() {
    let fx = fixture(Some((Mode::Paper, "skipped_late")), true, true);
    assert_eq!(run(&fx), RebalanceOutcome::SkippedLate);
}

// R-511
#[test]
fn an_earlier_dry_run_decision_does_not_count_so_the_rebalance_is_bootstrap() {
    let fx = fixture(Some((Mode::DryRun, "planned")), true, true);
    let outcome = run(&fx);
    assert!(
        matches!(outcome, RebalanceOutcome::Recorded { .. }),
        "got {outcome:?}"
    );
    let row = fx
        .store
        .decision_for("tsmom-v1", d(2024, 2, 29), Mode::Paper)
        .expect("lookup")
        .expect("row");
    assert!(
        record_from_json(&row.inputs_json)
            .expect("record")
            .bootstrap
    );
}

// R-512
#[test]
fn a_late_rebalance_with_a_missing_bar_is_data_not_ready_and_writes_no_decision() {
    let fx = fixture(Some((Mode::Paper, "planned")), false, true);
    assert_eq!(run(&fx), RebalanceOutcome::DataNotReady);
    assert!(fx
        .store
        .decision_for("tsmom-v1", d(2024, 2, 29), Mode::Paper)
        .expect("lookup")
        .is_none());
}

// R-512
#[test]
fn once_the_missing_bar_arrives_the_late_rebalance_is_skipped_late() {
    let fx = fixture(Some((Mode::Paper, "planned")), false, true);
    assert_eq!(run(&fx), RebalanceOutcome::DataNotReady);
    let rows: Vec<BarRow> = bar_rows()
        .into_iter()
        .filter(|(n, date, _)| is_missing_bar(n, *date))
        .map(|(_, _, r)| r)
        .collect();
    assert!(!rows.is_empty());
    fx.store.upsert_bars(&rows, Utc::now()).expect("bar");
    assert_eq!(run(&fx), RebalanceOutcome::SkippedLate);
}

// R-513
#[test]
fn data_not_ready_is_recorded_at_warn_level() {
    let fx = fixture(Some((Mode::Paper, "planned")), false, true);
    run(&fx);
    assert_eq!(alert_levels(&fx, "data_not_ready"), vec!["warn"]);
}

// R-513
#[test]
fn late_rebalance_skipped_is_recorded_at_warn_level() {
    let fx = fixture(Some((Mode::Paper, "planned")), true, true);
    assert_eq!(run(&fx), RebalanceOutcome::SkippedLate);
    assert_eq!(alert_levels(&fx, "late_rebalance_skipped"), vec!["warn"]);
}

// R-513
#[test]
fn not_tradable_is_recorded_at_warn_level() {
    let fx = fixture(None, true, false);
    let outcome = run(&fx);
    assert!(
        matches!(outcome, RebalanceOutcome::Recorded { .. }),
        "got {outcome:?}"
    );
    assert_eq!(alert_levels(&fx, "not_tradable"), vec!["warn"]);
}

// R-513
#[test]
fn a_skipped_late_rebalance_records_no_not_tradable_alert() {
    let fx = fixture(Some((Mode::Paper, "planned")), true, false);
    assert_eq!(run(&fx), RebalanceOutcome::SkippedLate);
    let row = fx
        .store
        .decision_for("tsmom-v1", d(2024, 2, 29), Mode::Paper)
        .expect("lookup")
        .expect("row");
    assert_eq!(row.status, "skipped_late");
    assert!(alert_levels(&fx, "not_tradable").is_empty());
}
