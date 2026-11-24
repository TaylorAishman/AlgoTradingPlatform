//! C11 (R-040): the config hash depends on values only.

use pt_config::config::{parse, Config};
use pt_config::hash::config_hash;

const UNIVERSE: &str =
    "[[universe]]\nsymbol = \"SPY\"\nasset_class = \"equity\"\nshortable = true\n";

fn cfg(text: &str) -> Config {
    parse(text).expect("config parses")
}

fn base() -> String {
    format!("{UNIVERSE}\n[sizing]\ntarget_vol = 0.10\nmax_weight = 0.25\nmax_gross = 0.5\n")
}

#[test]
fn hash_is_64_lowercase_hex_characters() {
    // R-040
    let h = config_hash(&cfg(&base()));
    assert_eq!(h.len(), 64);
    assert!(h
        .chars()
        .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c)));
}

#[test]
fn comments_and_blank_lines_do_not_change_the_hash() {
    // R-040
    let noisy = format!(
        "# leading comment\n\n\n{UNIVERSE}\n# about sizing\n[sizing]\n\ntarget_vol = 0.10 # vol\n\nmax_weight = 0.25\nmax_gross = 0.5\n\n"
    );
    assert_eq!(config_hash(&cfg(&base())), config_hash(&cfg(&noisy)));
}

#[test]
fn key_order_within_a_table_does_not_change_the_hash() {
    // R-040
    let reordered =
        format!("{UNIVERSE}\n[sizing]\nmax_gross = 0.5\nmax_weight = 0.25\ntarget_vol = 0.10\n");
    assert_eq!(config_hash(&cfg(&base())), config_hash(&cfg(&reordered)));
}

#[test]
fn changing_target_vol_changes_the_hash() {
    // R-040
    let changed = base().replace("target_vol = 0.10", "target_vol = 0.11");
    assert_ne!(config_hash(&cfg(&base())), config_hash(&cfg(&changed)));
}

#[test]
fn decimal_scale_does_not_change_the_hash() {
    // R-040: 0.10 and 0.1 are the same value
    let short = base().replace("target_vol = 0.10", "target_vol = 0.1");
    assert_eq!(config_hash(&cfg(&base())), config_hash(&cfg(&short)));
}

#[test]
fn hash_is_deterministic() {
    // R-040
    assert_eq!(config_hash(&cfg(&base())), config_hash(&cfg(&base())));
}
