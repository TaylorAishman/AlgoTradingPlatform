//! Encoding of values for storage.

use crate::error::StoreError;
use rust_decimal::Decimal;
use std::str::FromStr;

/// Encode a decimal as text so no precision is lost in storage.
pub fn dec_to_text(d: Decimal) -> String {
    d.to_string()
}

/// Decode text written by [`dec_to_text`].
pub fn text_to_dec(s: &str) -> Result<Decimal, StoreError> {
    Decimal::from_str(s).map_err(|e| StoreError::Codec(format!("invalid decimal {s:?}: {e}")))
}
