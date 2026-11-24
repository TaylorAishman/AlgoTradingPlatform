//! C15 (R-031): the cost section holds two CostSpec values, research and
//! shadow, whose keys default separately with the architecture 5.3 defaults.
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
fn cost_section_defaults_when_absent() {
    // R-031
    let c = with("").cost;
    assert_eq!(txt(c.research.spread_bps), "5");
    assert_eq!(txt(c.research.fee_bps), "0");
    assert_eq!(txt(c.research.impact_coeff), "1");
    assert_eq!(txt(c.research.borrow_bps_annual), "50");
    assert_eq!(txt(c.shadow.spread_bps), "5");
    assert_eq!(txt(c.shadow.fee_bps), "0");
    assert_eq!(txt(c.shadow.impact_coeff), "0");
    assert_eq!(txt(c.shadow.borrow_bps_annual), "0");
}

#[test]
fn research_and_shadow_default_separately() {
    // R-031: setting one spec leaves the other at its own defaults.
    let cfg = with("[cost.research]\nspread_bps = 2\n");
    assert_eq!(txt(cfg.cost.research.spread_bps), "2");
    assert_eq!(txt(cfg.cost.research.impact_coeff), "1");
    assert_eq!(txt(cfg.cost.research.borrow_bps_annual), "50");
    assert_eq!(txt(cfg.cost.shadow.impact_coeff), "0");
    assert_eq!(txt(cfg.cost.shadow.borrow_bps_annual), "0");

    let cfg = with("[cost.shadow]\nborrow_bps_annual = 25\n");
    assert_eq!(txt(cfg.cost.shadow.borrow_bps_annual), "25");
    assert_eq!(txt(cfg.cost.shadow.spread_bps), "5");
    assert_eq!(txt(cfg.cost.research.borrow_bps_annual), "50");
    assert_eq!(txt(cfg.cost.research.impact_coeff), "1");
}

#[test]
fn keys_default_individually_within_a_spec() {
    // R-031
    let cfg = with("[cost.research]\nfee_bps = 0.75\nimpact_coeff = 0.5\n");
    assert_eq!(txt(cfg.cost.research.fee_bps), "0.75");
    assert_eq!(txt(cfg.cost.research.impact_coeff), "0.5");
    assert_eq!(txt(cfg.cost.research.spread_bps), "5");
    assert_eq!(txt(cfg.cost.research.borrow_bps_annual), "50");
}

#[test]
fn given_values_override_defaults_exactly() {
    // R-031
    let cfg = with(
        "[cost.research]\nspread_bps = 1.1\nfee_bps = 0.3\nimpact_coeff = 2.5\nborrow_bps_annual = 75\n\
         [cost.shadow]\nspread_bps = 0.1\nfee_bps = 0.2\nimpact_coeff = 0.7\nborrow_bps_annual = 12\n",
    );
    assert_eq!(txt(cfg.cost.research.spread_bps), "1.1");
    assert_eq!(txt(cfg.cost.research.fee_bps), "0.3");
    assert_eq!(txt(cfg.cost.research.impact_coeff), "2.5");
    assert_eq!(txt(cfg.cost.research.borrow_bps_annual), "75");
    assert_eq!(txt(cfg.cost.shadow.spread_bps), "0.1");
    assert_eq!(txt(cfg.cost.shadow.fee_bps), "0.2");
    assert_eq!(txt(cfg.cost.shadow.impact_coeff), "0.7");
    assert_eq!(txt(cfg.cost.shadow.borrow_bps_annual), "12");
}

#[test]
fn unknown_cost_keys_are_rejected() {
    // R-031
    assert!(parse(&format!("{UNIVERSE}\n[cost.research]\nslippage = 1\n")).is_err());
    assert!(parse(&format!("{UNIVERSE}\n[cost.other]\nspread_bps = 1\n")).is_err());
}
