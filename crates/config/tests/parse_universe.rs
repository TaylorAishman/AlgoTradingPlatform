//! C12 (R-031): `parse` reads the universe of instruments with the six asset classes.
//! Values are compared through their printed text so no type is pinned.

use pt_config::config::parse;
use pt_config::error::ConfigError;

/// Printed text reduced to lowercase letters and digits, so `Equity`,
/// `"equity"` and `RealEstate` / `real_estate` compare alike.
fn norm(text: String) -> String {
    text.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

fn entry(symbol: &str, class: &str, shortable: bool) -> String {
    format!(
        "[[universe]]\nsymbol = \"{symbol}\"\nasset_class = \"{class}\"\nshortable = {shortable}\n"
    )
}

#[test]
fn each_of_the_six_asset_classes_is_read() {
    // R-031
    let classes = [
        ("SPY", "equity", "equity"),
        ("IEF", "treasury", "treasury"),
        ("LQD", "credit", "credit"),
        ("GLD", "commodity", "commodity"),
        ("UUP", "currency", "currency"),
        ("VNQ", "real_estate", "realestate"),
    ];
    for (symbol, class, expected) in classes {
        let cfg = parse(&entry(symbol, class, true)).expect("a lone universe entry parses");
        assert_eq!(cfg.universe.len(), 1, "{class}");
        let inst = &cfg.universe[0];
        assert_eq!(norm(format!("{:?}", inst.symbol)), symbol.to_lowercase());
        assert_eq!(norm(format!("{:?}", inst.asset_class)), expected, "{class}");
        assert_eq!(format!("{}", inst.shortable), "true");
    }
}

#[test]
fn shortable_false_is_read() {
    // R-031
    let cfg = parse(&entry("SPY", "equity", false)).expect("parses");
    assert_eq!(format!("{}", cfg.universe[0].shortable), "false");
}

#[test]
fn several_entries_keep_file_order() {
    // R-031
    let text = format!(
        "{}{}{}",
        entry("SPY", "equity", true),
        entry("TLT", "treasury", true),
        entry("VNQ", "real_estate", false)
    );
    let cfg = parse(&text).expect("parses");
    let symbols: Vec<String> = cfg
        .universe
        .iter()
        .map(|i| norm(format!("{:?}", i.symbol)))
        .collect();
    assert_eq!(symbols, ["spy", "tlt", "vnq"]);
    assert_eq!(format!("{}", cfg.universe[2].shortable), "false");
}

#[test]
fn an_unknown_asset_class_is_a_parse_error() {
    // R-031, R-037: the class is one of six
    let err = parse(&entry("BTC", "crypto", true)).expect_err("crypto is not a class");
    assert!(matches!(err, ConfigError::Parse(_)), "{err:?}");
}
