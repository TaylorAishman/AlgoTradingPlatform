//! K10 (R-197): `record_from_json(&record_to_json(&r))` returns a record equal to `r`.

mod common;

use chrono::{TimeZone, Utc};
use common::*;
use pt_core::money::Money;
use pt_core::types::Positions;
use pt_engine::decision::{record_from_json, record_to_json, DecisionInputs, DecisionRecord};
use pt_engine::error::EngineError;
use rust_decimal::Decimal;
use std::collections::BTreeSet;
use std::str::FromStr;

fn record(bootstrap: bool) -> DecisionRecord {
    DecisionRecord {
        inputs: DecisionInputs {
            signal_date: d(2024, 1, 2),
            trade_date: d(2024, 1, 3),
            ref_date: d(2024, 1, 2),
            equity: Money(Decimal::from_str("100000.125").unwrap()),
            positions: Positions::from([(sym("A"), 500), (sym("B"), -300)]),
            universe: vec![sym("B"), sym("A")],
            shortable: BTreeSet::from([sym("B")]),
        },
        data_asof: Utc.with_ymd_and_hms(2024, 1, 2, 21, 5, 7).unwrap(),
        feed: "iex".to_string(),
        config_hash: "abc123".to_string(),
        code_version: "0.1.0-test".to_string(),
        bootstrap,
    }
}

// R-197
#[test]
fn record_round_trips_through_json() {
    let r = record(false);
    assert_eq!(record_from_json(&record_to_json(&r)).unwrap(), r);
}

// R-197
#[test]
fn bootstrap_record_round_trips_through_json() {
    let r = record(true);
    assert_eq!(record_from_json(&record_to_json(&r)).unwrap(), r);
}

// R-197
#[test]
fn empty_positions_and_universe_round_trip() {
    let mut r = record(false);
    r.inputs.positions = Positions::new();
    r.inputs.universe = vec![];
    r.inputs.shortable = BTreeSet::new();
    assert_eq!(record_from_json(&record_to_json(&r)).unwrap(), r);
}

// R-197
#[test]
fn equal_records_give_equal_json_text() {
    assert_eq!(
        record_to_json(&record(false)),
        record_to_json(&record(false))
    );
}

// R-197
#[test]
fn malformed_json_is_an_invalid_error() {
    assert!(matches!(
        record_from_json("not json"),
        Err(EngineError::Invalid(_))
    ));
}
