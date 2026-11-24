use pt_core::error::CoreError;
use pt_core::types::Symbol;

// R-010
#[test]
fn accepts_plain_and_dotted_symbols_and_returns_input() {
    for s in ["SPY", "BRK.B", "A", "ABCDEFGHIJKL", "A1.2"] {
        let sym = Symbol::new(s).unwrap();
        assert_eq!(sym.as_str(), s);
    }
}

// R-010
#[test]
fn rejects_empty_lowercase_digit_first_space_and_too_long() {
    for s in ["", "spy", "1ABC", "SP Y", "ABCDEFGHIJKLM", ".AB", "AB-C"] {
        assert!(
            matches!(Symbol::new(s), Err(CoreError::InvalidSymbol(_))),
            "{s:?} should be rejected"
        );
    }
}

// R-010
#[test]
fn thirteen_characters_is_rejected_and_twelve_is_accepted() {
    assert_eq!("ABCDEFGHIJKLM".len(), 13);
    assert!(Symbol::new("ABCDEFGHIJKLM").is_err());
    assert!(Symbol::new("ABCDEFGHIJKL").is_ok());
}
