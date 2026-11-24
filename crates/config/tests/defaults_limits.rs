//! C14 (R-031): the sizing and risk sections read their architecture 5.3
//! defaults, and fractional values are exact decimals read from TOML numbers.
//! Values are compared through their printed text, with trailing fractional
//! zeros removed, so no type is pinned.

use pt_config::config::{parse, Config};

const UNIVERSE: &str =
    "[[universe]]\nsymbol = \"SPY\"\nasset_class = \"equity\"\nshortable = true\n";

fn with(extra: &str) -> Config {
    parse(&format!("{UNIVERSE}\n{extra}")).expect("config parses")
}

/// Printed text of a value with trailing fractional zeros removed.
fn txt<T: std::fmt::Display>(v: T) -> String {
    let s = v.to_string();
    if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        s
    }
}

#[test]
fn sizing_section_defaults_when_absent() {
    // R-031
    let s = with("").sizing;
    assert_eq!(txt(s.target_vol), "0.1");
    assert_eq!(txt(s.max_weight), "0.1");
    assert_eq!(txt(s.max_gross), "1");
}

#[test]
fn risk_section_defaults_when_absent() {
    // R-031
    let r = with("").risk;
    assert_eq!(txt(r.max_order_notional), "25000");
    assert_eq!(txt(r.daily_loss_limit), "0.03");
    assert_eq!(txt(r.max_drawdown), "0.15");
    assert_eq!(txt(r.max_rebalance_delay_sessions), "5");
    assert_eq!(txt(r.min_minutes_before_close), "30");
    assert_eq!(txt(r.cash_tolerance), "0.01");
}

#[test]
fn given_values_override_defaults() {
    // R-031
    let cfg = with(
        "[sizing]\ntarget_vol = 0.2\nmax_weight = 0.25\nmax_gross = 0.5\n\
         [risk]\nmax_order_notional = 1000\ndaily_loss_limit = 0.05\nmax_drawdown = 0.2\n\
         max_rebalance_delay_sessions = 3\nmin_minutes_before_close = 10\ncash_tolerance = 0.5\n",
    );
    assert_eq!(txt(cfg.sizing.target_vol), "0.2");
    assert_eq!(txt(cfg.sizing.max_weight), "0.25");
    assert_eq!(txt(cfg.sizing.max_gross), "0.5");
    assert_eq!(txt(cfg.risk.max_order_notional), "1000");
    assert_eq!(txt(cfg.risk.daily_loss_limit), "0.05");
    assert_eq!(txt(cfg.risk.max_drawdown), "0.2");
    assert_eq!(txt(cfg.risk.max_rebalance_delay_sessions), "3");
    assert_eq!(txt(cfg.risk.min_minutes_before_close), "10");
    assert_eq!(txt(cfg.risk.cash_tolerance), "0.5");
}

#[test]
fn fractional_values_read_back_exactly() {
    // R-031: no binary floating-point error survives the read.
    let cfg = with(
        "[sizing]\ntarget_vol = 0.11\nmax_gross = 0.99\n\
         [risk]\nmax_order_notional = 25000\ndaily_loss_limit = 0.1\n",
    );
    assert_eq!(txt(cfg.sizing.target_vol), "0.11");
    assert_eq!(txt(cfg.sizing.max_gross), "0.99");
    assert_eq!(txt(cfg.risk.max_order_notional), "25000");
    assert_eq!(txt(cfg.risk.daily_loss_limit), "0.1");
}

#[test]
fn one_key_set_keeps_the_rest_of_its_section_default() {
    // R-031
    let cfg = with("[risk]\nmax_drawdown = 0.2\n[sizing]\nmax_weight = 0.05\n");
    assert_eq!(txt(cfg.risk.max_drawdown), "0.2");
    assert_eq!(txt(cfg.risk.daily_loss_limit), "0.03");
    assert_eq!(txt(cfg.risk.max_order_notional), "25000");
    assert_eq!(txt(cfg.sizing.max_weight), "0.05");
    assert_eq!(txt(cfg.sizing.target_vol), "0.1");
    assert_eq!(txt(cfg.sizing.max_gross), "1");
}
