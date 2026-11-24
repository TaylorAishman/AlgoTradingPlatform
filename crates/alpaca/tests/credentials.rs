use pt_alpaca::auth::Credentials;

// R-083: credentials never print in Debug output.
#[test]
fn debug_output_contains_neither_key_id_nor_secret() {
    let creds = Credentials::new("KEYID".into(), "TOPSECRET".into());
    let text = format!("{:?}", creds);
    assert!(!text.contains("KEYID"), "leaked key id: {text}");
    assert!(!text.contains("TOPSECRET"), "leaked secret: {text}");
}

// R-083: the pretty form is redacted too.
#[test]
fn pretty_debug_output_is_redacted() {
    let creds = Credentials::new("KEYID".into(), "TOPSECRET".into());
    let text = format!("{:#?}", creds);
    assert!(!text.contains("KEYID"), "leaked key id: {text}");
    assert!(!text.contains("TOPSECRET"), "leaked secret: {text}");
}

// R-083: Debug prints the redaction marker.
#[test]
fn debug_output_shows_the_redaction_marker() {
    let creds = Credentials::new("KEYID".into(), "TOPSECRET".into());
    assert!(format!("{:?}", creds).contains("***"));
}
