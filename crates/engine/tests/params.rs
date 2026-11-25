//! K05 (R-212): `params_from_config` is the one place config becomes decision
//! parameters.

use pt_config::config::{parse, Config};
use pt_engine::decision::params_from_config;
use rust_decimal::Decimal;

const UNIVERSE: &str =
    "[[universe]]\nsymbol = \"SPY\"\nasset_class = \"equity\"\nshortable = true\n";

fn config(extra: &str) -> Config {
    parse(&format!("{UNIVERSE}\n{extra}")).expect("valid config")
}

fn dec(mantissa: i64, scale: u32) -> Decimal {
    Decimal::new(mantissa, scale)
}

#[test]
fn default_config_gives_the_volatility_parameters() {
    // R-212
    let p = params_from_config(&config(""));
    assert_eq!(p.vol.com_days, 60.0);
    assert_eq!(p.vol.annualization, 261.0);
    assert_eq!(p.vol.min_obs, 126);
    assert_eq!(p.vol.max_obs, 504);
}

#[test]
fn default_config_gives_the_sizing_parameters() {
    // R-212
    let p = params_from_config(&config(""));
    assert_eq!(p.sizing.target_vol, 0.10);
    assert_eq!(p.sizing.max_weight, 0.10);
    assert_eq!(p.sizing.max_gross, 1.0);
}

#[test]
fn default_config_gives_the_risk_limits() {
    // R-212
    let p = params_from_config(&config(""));
    assert_eq!(p.limits.max_order_notional, dec(25000, 0));
    assert_eq!(p.limits.max_weight, dec(10, 2));
    assert_eq!(p.limits.max_gross, dec(1, 0));
    assert_eq!(p.limits.daily_loss_limit, dec(3, 2));
    assert_eq!(p.limits.max_drawdown, dec(15, 2));
}

#[test]
fn sizing_max_weight_feeds_both_sizing_and_limits() {
    // R-212
    let p = params_from_config(&config("[sizing]\nmax_weight = 0.2\n"));
    assert_eq!(p.sizing.max_weight, 0.2);
    assert_eq!(p.limits.max_weight, dec(2, 1));
}
