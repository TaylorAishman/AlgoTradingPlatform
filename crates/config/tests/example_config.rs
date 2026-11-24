//! C10 (R-030, R-039): `load` reads the committed example config and its universe.

use std::path::{Path, PathBuf};

use pt_config::config::{load, AssetClass};
use pt_config::error::ConfigError;

fn example_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../config/paper-trader.example.toml")
}

#[test]
fn example_config_loads_with_default_strategy_and_feed() {
    // R-030
    let cfg = load(&example_path()).expect("example config must load and validate");
    assert_eq!(cfg.strategy.id, "tsmom-v1");
    assert_eq!(cfg.data.feed, "sip");
}

#[test]
fn example_universe_is_the_37_candidates_with_their_classes() {
    // R-039
    let cfg = load(&example_path()).expect("example config must load");
    let expected: [(AssetClass, &[&str]); 6] = [
        (
            AssetClass::Equity,
            &[
                "SPY", "QQQ", "IWM", "MDY", "EFA", "EEM", "VGK", "EWJ", "EWU", "EWG", "EWC", "EWA",
                "EWZ", "FXI", "INDA",
            ],
        ),
        (
            AssetClass::Treasury,
            &["SHY", "IEI", "IEF", "TLH", "TLT", "TIP"],
        ),
        (AssetClass::Credit, &["LQD", "HYG", "EMB"]),
        (
            AssetClass::Commodity,
            &["GLD", "SLV", "DBC", "USO", "UNG", "DBA"],
        ),
        (AssetClass::Currency, &["UUP", "FXE", "FXY", "FXB", "FXA"]),
        (AssetClass::RealEstate, &["VNQ", "RWX"]),
    ];
    let mut want: Vec<(String, AssetClass)> = expected
        .iter()
        .flat_map(|(class, syms)| syms.iter().map(move |s| (s.to_string(), *class)))
        .collect();
    let mut got: Vec<(String, AssetClass)> = cfg
        .universe
        .iter()
        .map(|i| (i.symbol.clone(), i.asset_class))
        .collect();
    assert_eq!(want.len(), 37);
    want.sort_by(|a, b| a.0.cmp(&b.0));
    got.sort_by(|a, b| a.0.cmp(&b.0));
    assert_eq!(got, want);
}

#[test]
fn every_example_instrument_is_shortable() {
    // R-039
    let cfg = load(&example_path()).expect("example config must load");
    assert!(cfg.universe.iter().all(|i| i.shortable));
}

#[test]
fn load_of_a_missing_file_is_an_io_error() {
    // R-030
    let dir = tempfile::tempdir().expect("temp dir");
    let err = load(&dir.path().join("absent.toml")).expect_err("missing file");
    assert!(matches!(err, ConfigError::Io(_)), "got {err:?}");
}

#[test]
fn load_applies_validation_to_the_file_contents() {
    // R-030
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("empty.toml");
    std::fs::write(&path, "").expect("write");
    let err = load(&path).expect_err("empty universe is invalid");
    assert!(matches!(err, ConfigError::Invalid(_)), "got {err:?}");
}

#[test]
fn load_reports_malformed_toml_as_a_parse_error() {
    // R-030
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("bad.toml");
    std::fs::write(&path, "this is = = not toml").expect("write");
    let err = load(&path).expect_err("malformed");
    assert!(matches!(err, ConfigError::Parse(_)), "got {err:?}");
}
