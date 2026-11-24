use pt_store::codec::{dec_to_text, text_to_dec};
use pt_store::error::StoreError;
use rust_decimal::Decimal;
use std::str::FromStr;

fn roundtrip(s: &str) {
    let d = Decimal::from_str(s).unwrap();
    assert_eq!(text_to_dec(&dec_to_text(d)).unwrap(), d, "value {s}");
}

// R-053
#[test]
fn long_fraction_survives_storage() {
    roundtrip("123.456789012345678");
}

// R-053
#[test]
fn small_negative_survives_storage() {
    roundtrip("-0.000001");
}

// R-053
#[test]
fn whole_number_survives_storage() {
    roundtrip("100000");
}

// R-053
#[test]
fn non_numeric_text_is_a_codec_error() {
    assert!(matches!(text_to_dec("abc"), Err(StoreError::Codec(_))));
}
