//! C09 (R-038): `parse` range-checks the strategy, sizing, risk and statistics
//! parameters. Failures are `ConfigError::Invalid` and the message names the key.

use pt_config::config::{parse, Config};
use pt_config::error::ConfigError;

const UNIVERSE: &str =
    "[[universe]]\nsymbol = \"SPY\"\nasset_class = \"equity\"\nshortable = true\n";

fn try_with(extra: &str) -> Result<Config, ConfigError> {
    parse(&format!("{UNIVERSE}\n{extra}"))
}

fn invalid_message(extra: &str) -> String {
    match try_with(extra) {
        Err(ConfigError::Invalid(m)) => m,
        other => panic!("expected ConfigError::Invalid, got {other:?}"),
    }
}

#[test]
fn empty_lookback_months_is_invalid() {
    // R-038
    let m = invalid_message("[strategy]\nlookback_months = []\n");
    assert!(m.contains("lookback_months"), "message was: {m}");
}

#[test]
fn zero_lookback_month_is_invalid() {
    // R-038
    let m = invalid_message("[strategy]\nlookback_months = [0]\n");
    assert!(m.contains("lookback_months"), "message was: {m}");
}

#[test]
fn duplicate_lookback_months_are_invalid() {
    // R-038
    let m = invalid_message("[strategy]\nlookback_months = [12, 12]\n");
    assert!(m.contains("lookback_months"), "message was: {m}");
}

#[test]
fn distinct_positive_lookback_months_are_accepted() {
    // R-038
    assert!(try_with("[strategy]\nlookback_months = [1, 3, 12]\n").is_ok());
    assert!(try_with("[strategy]\nlookback_months = [12]\n").is_ok());
}

#[test]
fn vol_min_obs_above_default_vol_max_obs_is_invalid() {
    // R-038
    let m = invalid_message("[strategy]\nvol_min_obs = 600\n");
    assert!(m.contains("vol_min_obs"), "message was: {m}");
}

#[test]
fn vol_min_obs_with_a_larger_vol_max_obs_is_accepted() {
    // R-038
    assert!(try_with("[strategy]\nvol_min_obs = 600\nvol_max_obs = 700\n").is_ok());
}

#[test]
fn pbo_partitions_of_fifteen_is_invalid() {
    // R-038
    let m = invalid_message("[stats]\npbo_partitions = 15\n");
    assert!(m.contains("pbo_partitions"), "message was: {m}");
}

#[test]
fn pbo_partitions_of_sixteen_is_accepted() {
    // R-038
    assert!(try_with("[stats]\npbo_partitions = 16\n").is_ok());
}

#[test]
fn zero_target_vol_is_invalid() {
    // R-038
    let m = invalid_message("[sizing]\ntarget_vol = 0\n");
    assert!(m.contains("target_vol"), "message was: {m}");
}

#[test]
fn negative_target_vol_is_invalid() {
    // R-038
    let m = invalid_message("[sizing]\ntarget_vol = -0.10\n");
    assert!(m.contains("target_vol"), "message was: {m}");
}

#[test]
fn positive_target_vol_is_accepted() {
    // R-038
    assert!(try_with("[sizing]\ntarget_vol = 0.10\n").is_ok());
}

#[test]
fn zero_max_order_notional_is_invalid() {
    // R-038
    let m = invalid_message("[risk]\nmax_order_notional = 0\n");
    assert!(m.contains("max_order_notional"), "message was: {m}");
}

#[test]
fn negative_max_order_notional_is_invalid() {
    // R-038
    let m = invalid_message("[risk]\nmax_order_notional = -1\n");
    assert!(m.contains("max_order_notional"), "message was: {m}");
}

#[test]
fn positive_max_order_notional_is_accepted() {
    // R-038
    assert!(try_with("[risk]\nmax_order_notional = 1\n").is_ok());
}

#[test]
fn fraction_keys_must_be_in_zero_exclusive_one_inclusive() {
    // R-038
    for (block, key) in [
        ("sizing", "max_weight"),
        ("risk", "daily_loss_limit"),
        ("risk", "max_drawdown"),
    ] {
        for bad in ["0", "-0.1", "1.01"] {
            let m = invalid_message(&format!("[{block}]\n{key} = {bad}\n"));
            assert!(m.contains(&format!("{block}.{key}")), "message was: {m}");
        }
        for good in ["1.0", "0.0001"] {
            assert!(try_with(&format!("[{block}]\n{key} = {good}\n")).is_ok());
        }
    }
}

#[test]
fn negative_cash_tolerance_is_invalid_and_zero_is_accepted() {
    // R-038
    let m = invalid_message("[risk]\ncash_tolerance = -0.01\n");
    assert!(m.contains("risk.cash_tolerance"), "message was: {m}");
    assert!(try_with("[risk]\ncash_tolerance = 0\n").is_ok());
}

#[test]
fn count_keys_must_be_positive() {
    // R-038
    for (block, key) in [
        ("stats", "periods_per_year"),
        ("strategy", "vol_com_days"),
        ("strategy", "vol_annualization"),
    ] {
        let m = invalid_message(&format!("[{block}]\n{key} = 0\n"));
        assert!(m.contains(&format!("{block}.{key}")), "message was: {m}");
        assert!(try_with(&format!("[{block}]\n{key} = 1\n")).is_ok());
    }
}

#[test]
fn negative_research_spread_bps_is_invalid() {
    // R-038
    let m = invalid_message("[cost.research]\nspread_bps = -1\n");
    assert!(m.contains("cost.research"), "message was: {m}");
}

#[test]
fn negative_shadow_impact_coeff_is_invalid() {
    // R-038
    let m = invalid_message("[cost.shadow]\nimpact_coeff = -0.5\n");
    assert!(m.contains("cost.shadow"), "message was: {m}");
}

#[test]
fn negative_research_impact_coeff_is_invalid() {
    // R-038
    let m = invalid_message("[cost.research]\nimpact_coeff = -1\n");
    assert!(m.contains("cost.research"), "message was: {m}");
}

#[test]
fn negative_shadow_spread_bps_is_invalid() {
    // R-038
    let m = invalid_message("[cost.shadow]\nspread_bps = -1\n");
    assert!(m.contains("cost.shadow"), "message was: {m}");
}

#[test]
fn lowercase_universe_symbol_is_invalid() {
    // R-038
    let text = "[[universe]]\nsymbol = \"spy\"\nasset_class = \"equity\"\nshortable = true\n";
    match parse(text) {
        Err(ConfigError::Invalid(m)) => assert!(m.contains("spy"), "message was: {m}"),
        other => panic!("expected ConfigError::Invalid, got {other:?}"),
    }
}

#[test]
fn malformed_risk_free_symbol_is_invalid() {
    // R-038
    let m = invalid_message("[strategy]\nrisk_free_symbol = \"bil\"\n");
    assert!(m.contains("risk_free_symbol"), "message was: {m}");
}

#[test]
fn default_parameters_are_valid() {
    // R-038: the defaults pass every range check.
    assert!(try_with("").is_ok());
}
