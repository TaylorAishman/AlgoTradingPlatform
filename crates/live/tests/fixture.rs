//! R-283 (foundation): the live context borrows a seeded fixture, and the fixture matches
//! the live fixture of the requirements. `submit_planned` itself is covered by its own step.

mod common;

use common::{d, LiveFixture};
use pt_alpaca::account::account;
use pt_alpaca::reference::{asset, clock};
use pt_gate::owner::Stage;
use rust_decimal::Decimal;

// R-283
#[test]
fn context_exposes_the_fixture_store_config_gate_and_strategy() {
    let fx = LiveFixture::new(Stage::Paper);
    let ctx = fx.ctx();
    assert_eq!(ctx.gate.stage(), Stage::Paper);
    assert_eq!(ctx.strategy.id(), "tsmom-v1");
    assert_eq!(ctx.code_version, "test-version");
    assert_eq!(ctx.clock.now_utc(), fx.now());
    let universe: Vec<&str> = ctx.cfg.universe.iter().map(|i| i.symbol.as_str()).collect();
    assert_eq!(universe, ["SPY", "TLT"]);
    assert_eq!(ctx.cfg.strategy.risk_free_symbol, "BIL");
    assert_eq!(ctx.cfg.risk.max_order_notional, Decimal::from(60000));
    let cal = ctx.store.calendar().expect("calendar");
    assert_eq!(
        cal.sessions_between(d(2024, 1, 1), d(2024, 12, 31)).len(),
        252
    );
}

// R-283
#[test]
fn context_gate_follows_the_stage_named_by_the_test() {
    for stage in [Stage::DryRun, Stage::Ingest, Stage::FirstOrder] {
        assert_eq!(LiveFixture::new(stage).ctx().gate.stage(), stage);
    }
}

// R-283
#[test]
fn context_http_reads_the_seeded_account_clock_and_assets() {
    let fx = LiveFixture::new(Stage::Paper);
    let ctx = fx.ctx();
    assert_eq!(
        account(ctx.http).expect("account").cash,
        Decimal::from(100000)
    );
    let c = clock(ctx.http).expect("clock");
    assert!(!c.is_open);
    assert_eq!(c.timestamp.to_rfc3339(), "2024-03-01T08:00:00-05:00");
    for s in ["SPY", "TLT", "BIL"] {
        let a = asset(ctx.http, s).expect("asset").expect("seeded");
        assert!(a.tradable && a.shortable, "{s}");
    }
    assert!(fx.mock.requests().iter().all(|r| r.method == "GET"));
}
