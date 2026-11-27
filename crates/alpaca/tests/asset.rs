mod common;

use std::sync::Arc;

use pt_alpaca::auth::Credentials;
use pt_alpaca::endpoints::Endpoints;
use pt_alpaca::http::HttpClient;
use pt_alpaca::limiter::{MonoClock, RateLimiter, Sleeper};
use pt_alpaca::reference::asset;

use common::{Fake, Listener};

const BOOL_FIELD_BODY: &str = include_str!("fixtures/asset.json");

fn client(url: &str, fake: &Arc<Fake>) -> HttpClient {
    let mono: Arc<dyn MonoClock> = fake.clone();
    let sleeper: Arc<dyn Sleeper> = fake.clone();
    let limiter = RateLimiter::new(200, mono, sleeper.clone()).expect("a limit of 200 is accepted");
    let endpoints = Endpoints::with_overrides(Some(url), Some(url)).expect("loopback");
    HttpClient::new(
        endpoints,
        Credentials::new("FAKEKEYID".to_string(), "FAKESECRET".to_string()),
        limiter,
        sleeper,
    )
}

// R-096: the boolean easy_to_borrow field is read.
#[test]
fn boolean_field_gives_easy_to_borrow() {
    let l = Listener::serve(vec![(200, BOOL_FIELD_BODY.to_string())]);
    let fake = Fake::new();
    let http = client(&l.url, &fake);
    let a = asset(&http, "SPY").expect("asset").expect("some");
    let seen = l.finish();
    assert_eq!(seen[0].method, "GET");
    assert_eq!(seen[0].path(), "/v2/assets/SPY");
    assert_eq!(a.symbol, "SPY");
    assert!(a.tradable);
    assert!(a.shortable);
    assert!(a.easy_to_borrow);
}

// R-096: borrow_status "easy_to_borrow" also gives easy_to_borrow.
#[test]
fn borrow_status_easy_gives_easy_to_borrow() {
    let body =
        r#"{"symbol":"SPY","tradable":true,"shortable":true,"borrow_status":"easy_to_borrow"}"#;
    let l = Listener::serve(vec![(200, body.to_string())]);
    let fake = Fake::new();
    let http = client(&l.url, &fake);
    let a = asset(&http, "SPY").expect("asset").expect("some");
    l.finish();
    assert!(a.easy_to_borrow);
}

// R-096: borrow_status "hard_to_borrow" with no boolean field is not easy to borrow.
#[test]
fn borrow_status_hard_is_not_easy_to_borrow() {
    let body =
        r#"{"symbol":"SPY","tradable":true,"shortable":true,"borrow_status":"hard_to_borrow"}"#;
    let l = Listener::serve(vec![(200, body.to_string())]);
    let fake = Fake::new();
    let http = client(&l.url, &fake);
    let a = asset(&http, "SPY").expect("asset").expect("some");
    l.finish();
    assert!(!a.easy_to_borrow);
    assert!(a.shortable);
}

// R-096: an asset with neither borrow field is not easy to borrow.
#[test]
fn no_borrow_field_is_not_easy_to_borrow() {
    let body = r#"{"symbol":"SPY","tradable":true,"shortable":true}"#;
    let l = Listener::serve(vec![(200, body.to_string())]);
    let fake = Fake::new();
    let http = client(&l.url, &fake);
    let a = asset(&http, "SPY").expect("asset").expect("some");
    l.finish();
    assert!(!a.easy_to_borrow);
}

// R-096: an unknown symbol (404) is Ok(None).
#[test]
fn unknown_asset_is_none() {
    let l = Listener::serve(vec![(404, r#"{"message":"asset not found"}"#.to_string())]);
    let fake = Fake::new();
    let http = client(&l.url, &fake);
    let a = asset(&http, "NOPE").expect("404 is not an error");
    let seen = l.finish();
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].path(), "/v2/assets/NOPE");
    assert!(a.is_none());
}

// R-435: a slash in the symbol is percent-encoded in the request path.
#[test]
fn symbol_with_slash_is_percent_encoded_in_path() {
    let body = r#"{"symbol":"BRK/B","tradable":true,"shortable":true}"#;
    let l = Listener::serve(vec![(200, body.to_string())]);
    let fake = Fake::new();
    let http = client(&l.url, &fake);
    asset(&http, "BRK/B").expect("asset").expect("some");
    let seen = l.finish();
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].method, "GET");
    assert_eq!(seen[0].path(), "/v2/assets/BRK%2FB");
}

// R-435: characters that would end or alter the path are encoded too.
#[test]
fn symbol_with_query_and_space_characters_is_encoded() {
    let l = Listener::serve(vec![(404, r#"{"message":"asset not found"}"#.to_string())]);
    let fake = Fake::new();
    let http = client(&l.url, &fake);
    asset(&http, "A B?c").expect("404 is not an error");
    let seen = l.finish();
    assert_eq!(seen[0].path(), "/v2/assets/A%20B%3Fc");
    assert_eq!(seen[0].query(), "");
}
