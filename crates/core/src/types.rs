//! Core domain types.

use std::fmt;

use crate::error::CoreError;

/// Longest accepted symbol, in characters.
const MAX_SYMBOL_LEN: usize = 12;

/// A validated ticker symbol: 1 to 12 characters of `A-Z`, `0-9` and `.`,
/// starting with a letter.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Symbol(String);

impl Symbol {
    /// Validate and wrap a symbol.
    pub fn new(s: &str) -> Result<Self, CoreError> {
        let valid = s.len() <= MAX_SYMBOL_LEN
            && s.chars().next().is_some_and(|c| c.is_ascii_uppercase())
            && s.chars()
                .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '.');
        if valid {
            Ok(Self(s.to_string()))
        } else {
            Err(CoreError::InvalidSymbol(s.to_string()))
        }
    }

    /// The symbol text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Symbol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
