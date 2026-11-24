//! Tests for deterministic order identifiers.

use chrono::NaiveDate;
use pt_live::ids::{client_order_id, first_order_id};

fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).expect("valid date")
}

// R-280
#[test]
fn client_order_id_has_the_documented_format() {
    assert_eq!(
        client_order_id("tsmom-v1", date(2026, 11, 30), "SPY", 1),
        "pt-tsmom-v1-20261130-SPY-1"
    );
}

// R-280
#[test]
fn client_order_id_is_deterministic() {
    let a = client_order_id("tsmom-v1", date(2026, 11, 30), "SPY", 1);
    let b = client_order_id("tsmom-v1", date(2026, 11, 30), "SPY", 1);
    assert_eq!(a, b);
}

// R-280
#[test]
fn leg_two_ends_with_dash_two() {
    let id = client_order_id("tsmom-v1", date(2026, 11, 30), "SPY", 2);
    assert!(id.ends_with("-2"), "got {id}");
    assert_ne!(
        id,
        client_order_id("tsmom-v1", date(2026, 11, 30), "SPY", 1)
    );
}

// R-280
#[test]
fn twelve_character_symbol_fits_in_128_characters() {
    let id = client_order_id("tsmom-v1", date(2026, 11, 30), "ABCDEFGHIJKL", 1);
    assert!(id.len() <= 128, "length {}", id.len());
    assert!(id.contains("ABCDEFGHIJKL"));
}

// R-280
#[test]
fn first_order_id_has_the_documented_format() {
    assert_eq!(
        first_order_id(date(2026, 12, 2), "SPY"),
        "pt-first-20261202-SPY"
    );
}
