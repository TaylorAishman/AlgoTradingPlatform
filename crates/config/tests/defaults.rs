//! R-031: omitted config keys take the defaults of architecture 5.3.
//!
//! Values are compared through their `Display` text parsed as a number, so the
//! tests do not depend on whether a field is a float, a decimal or an integer.

use pt_config::config::parse;

const MINIMAL: &str = r#"
[[universe]]
symbol = "SPY"
asset_class = "equity"
shortable = true
"#;

fn num<T: std::fmt::Display>(v: T) -> f64 {
    let s = v.to_string();
    s.parse::<f64>()
        .unwrap_or_else(|_| panic!("not numeric: {s}"))
}

// R-031
#[test]
fn minimal_config_parses_with_one_universe_entry() {
    let cfg = parse(MINIMAL).expect("minimal config must parse");
    assert_eq!(cfg.universe.len(), 1);
}

// R-031
#[test]
fn strategy_defaults() {
    let cfg = parse(MINIMAL).unwrap();
    let s = &cfg.strategy;
    assert_eq!(format!("{:?}", s.lookback_months), "[12]");
    assert_eq!(s.risk_free_symbol.to_string(), "BIL");
    assert_eq!(num(s.vol_com_days), 60.0);
    assert_eq!(num(s.vol_annualization), 261.0);
    assert_eq!(num(s.vol_min_obs), 126.0);
    assert_eq!(num(s.vol_max_obs), 504.0);
}

// R-031
#[test]
fn sizing_defaults() {
    let cfg = parse(MINIMAL).unwrap();
    assert_eq!(num(cfg.sizing.target_vol), 0.10);
    assert_eq!(num(cfg.sizing.max_weight), 0.10);
    assert_eq!(num(cfg.sizing.max_gross), 1.0);
}

// R-031
#[test]
fn risk_defaults() {
    let cfg = parse(MINIMAL).unwrap();
    let r = &cfg.risk;
    assert_eq!(num(r.max_order_notional), 25000.0);
    assert_eq!(num(r.daily_loss_limit), 0.03);
    assert_eq!(num(r.max_drawdown), 0.15);
    assert_eq!(num(r.max_rebalance_delay_sessions), 5.0);
    assert_eq!(num(r.min_minutes_before_close), 30.0);
    assert_eq!(num(r.cash_tolerance), 0.01);
}

// R-031
#[test]
fn cost_defaults() {
    let cfg = parse(MINIMAL).unwrap();
    assert_eq!(num(cfg.cost.research.spread_bps), 5.0);
    assert_eq!(num(cfg.cost.research.fee_bps), 0.0);
    assert_eq!(num(cfg.cost.research.impact_coeff), 1.0);
    assert_eq!(num(cfg.cost.research.borrow_bps_annual), 50.0);
    assert_eq!(num(cfg.cost.shadow.spread_bps), 5.0);
    assert_eq!(num(cfg.cost.shadow.fee_bps), 0.0);
    assert_eq!(num(cfg.cost.shadow.impact_coeff), 0.0);
    assert_eq!(num(cfg.cost.shadow.borrow_bps_annual), 0.0);
}

// R-031
#[test]
fn stats_and_data_defaults() {
    let cfg = parse(MINIMAL).unwrap();
    assert_eq!(num(cfg.stats.periods_per_year), 252.0);
    assert_eq!(num(cfg.stats.pbo_partitions), 16.0);
    assert_eq!(num(cfg.data.max_requests_per_minute), 180.0);
    assert_eq!(cfg.data.feed.to_string(), "sip");
    assert_eq!(cfg.data.start.to_string(), "2016-01-04");
}

// R-031
#[test]
fn run_defaults() {
    let cfg = parse(MINIMAL).unwrap();
    assert_eq!(cfg.run.db_path.to_string(), "var/paper-trader.sqlite");
    assert_eq!(cfg.run.output_dir.to_string(), "var/reports");
    assert_eq!(cfg.strategy.id.to_string(), "tsmom-v1");
}

// R-031
#[test]
fn signoff_defaults() {
    let cfg = parse(MINIMAL).unwrap();
    let g = &cfg.signoff;
    assert_eq!(num(g.min_sessions), 63.0);
    assert_eq!(num(g.min_rebalances), 3.0);
    assert_eq!(num(g.min_fills), 30.0);
    assert_eq!(num(g.max_replay_mismatches), 0.0);
    assert_eq!(num(g.max_reconciliation_breaks), 0.0);
    assert_eq!(num(g.max_missed_sessions), 2.0);
    assert_eq!(num(g.max_late_rebalances), 0.0);
    assert_eq!(num(g.slippage_tolerance_bps), 5.0);
    assert_eq!(num(g.max_mean_diff_bps), 1.0);
    assert_eq!(num(g.max_abs_diff_bps), 25.0);
}

// R-031
#[test]
fn explicit_value_overrides_default() {
    let text = format!("[strategy]\nlookback_months = [1, 3, 12]\n{MINIMAL}");
    let cfg = parse(&text).unwrap();
    assert_eq!(format!("{:?}", cfg.strategy.lookback_months), "[1, 3, 12]");
    assert_eq!(num(cfg.strategy.vol_com_days), 60.0);
}
