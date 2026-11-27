mod common;

use std::sync::Arc;

use pt_alpaca::auth::Credentials;
use pt_alpaca::endpoints::Endpoints;
use pt_alpaca::error::AlpacaError;
use pt_alpaca::http::HttpClient;
use pt_alpaca::limiter::{MonoClock, RateLimiter, Sleeper};
use pt_alpaca::orders_read::order_by_client_id;

use common::{Fake, Listener};

const ORDER: &str = include_str!("fixtures/order.json");

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

fn parse_message_for(order_json: String) -> String {
    let l = Listener::serve(vec![(200, order_json)]);
    let fake = Fake::new();
    let http = client(&l.url, &fake);
    let result = order_by_client_id(&http, "pt-x-1");
    l.finish();
    match result {
        Err(AlpacaError::Parse(m)) => m,
        other => panic!("expected AlpacaError::Parse, got {other:?}"),
    }
}

fn with_qty(qty: serde_json::Value) -> String {
    let mut v: serde_json::Value = serde_json::from_str(ORDER).expect("fixture is json");
    v["qty"] = qty;
    v["notional"] = serde_json::json!("100.00");
    v.to_string()
}

// R-437: an order with qty null and a notional is a parse error naming qty.
#[test]
fn null_qty_with_notional_is_a_parse_error_naming_qty() {
    let m = parse_message_for(with_qty(serde_json::Value::Null));
    assert!(m.contains("qty"), "message was {m:?}");
}

// R-437: an order with a fractional qty is a parse error naming qty.
#[test]
fn fractional_qty_is_a_parse_error_naming_qty() {
    let m = parse_message_for(with_qty(serde_json::json!("1.5")));
    assert!(m.contains("qty"), "message was {m:?}");
}
