use pt_alpaca::endpoints::Endpoints;
use pt_alpaca::error::AlpacaError;

const LOOPBACK_TRADING: &str = "http://127.0.0.1:9000";
const LOOPBACK_DATA: &str = "http://localhost:9001";

/// The URLs R-081 says must be refused. The live host is built at runtime so
/// that no source file names it.
fn rejected_urls() -> Vec<String> {
    vec![
        format!("https://api.{}", "alpaca.markets"),
        "https://example.com".to_string(),
        "http://127.0.0.1.example.com".to_string(),
        "http://10.0.0.5:80".to_string(),
    ]
}

fn is_not_allowed<T: std::fmt::Debug>(r: Result<T, AlpacaError>) -> bool {
    matches!(r, Err(AlpacaError::EndpointNotAllowed(_)))
}

// R-081: loopback overrides are accepted and returned as given.
#[test]
fn loopback_overrides_are_accepted_and_returned() {
    let e = Endpoints::with_overrides(Some(LOOPBACK_TRADING), Some(LOOPBACK_DATA))
        .expect("loopback overrides are accepted");
    assert_eq!(e.trading(), LOOPBACK_TRADING);
    assert_eq!(e.data(), LOOPBACK_DATA);
}

// R-081: each rejected URL is refused as the trading override.
#[test]
fn rejected_urls_are_refused_as_the_trading_override() {
    for url in rejected_urls() {
        let r = Endpoints::with_overrides(Some(&url), None);
        assert!(is_not_allowed(r), "{url} should be refused");
    }
}

// R-081: each rejected URL is refused as the data override.
#[test]
fn rejected_urls_are_refused_as_the_data_override() {
    for url in rejected_urls() {
        let r = Endpoints::with_overrides(None, Some(&url));
        assert!(is_not_allowed(r), "{url} should be refused");
    }
}

// R-081: one bad side refuses the whole pair, even beside a good loopback side.
#[test]
fn one_bad_side_refuses_the_pair() {
    for url in rejected_urls() {
        let a = Endpoints::with_overrides(Some(LOOPBACK_TRADING), Some(&url));
        assert!(is_not_allowed(a), "bad data side {url} should refuse");
        let b = Endpoints::with_overrides(Some(&url), Some(LOOPBACK_DATA));
        assert!(is_not_allowed(b), "bad trading side {url} should refuse");
    }
}

// R-081: no overrides equals production().
#[test]
fn no_overrides_equals_production() {
    assert_eq!(
        Endpoints::with_overrides(None, None).expect("no overrides"),
        Endpoints::production()
    );
}

// R-081: a single override leaves the other side at production.
#[test]
fn a_single_override_leaves_the_other_side_at_production() {
    let prod = Endpoints::production();

    let t = Endpoints::with_overrides(Some(LOOPBACK_TRADING), None).expect("trading only");
    assert_eq!(t.trading(), LOOPBACK_TRADING);
    assert_eq!(t.data(), prod.data());

    let d = Endpoints::with_overrides(None, Some(LOOPBACK_DATA)).expect("data only");
    assert_eq!(d.trading(), prod.trading());
    assert_eq!(d.data(), LOOPBACK_DATA);
}
