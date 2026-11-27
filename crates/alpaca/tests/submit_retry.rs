mod common;

use std::sync::Arc;

use chrono::NaiveDate;
use pt_alpaca::auth::Credentials;
use pt_alpaca::endpoints::Endpoints;
use pt_alpaca::error::AlpacaError;
use pt_alpaca::http::HttpClient;
use pt_alpaca::limiter::{MonoClock, RateLimiter, Sleeper};
use pt_alpaca::orders_write::{submit_order, NewOrder};
use pt_gate::gate::{AuthContext, Gate, OrderPermit, OrderSummary};
use pt_gate::owner::{OwnerFile, Stage};

use common::{Fake, Listener};

const REPLY: &str = r#"{"id":"b9","client_order_id":"pt-a-1","symbol":"SPY","side":"buy","qty":"10","filled_qty":"0","status":"accepted","submitted_at":"2026-12-02T14:30:00Z"}"#;

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

fn day(d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 12, d).unwrap()
}

fn permit() -> OrderPermit {
    let gate = Gate::new(OwnerFile {
        stage: Stage::Paper,
        strategy_allowlist: vec!["fake-strategy".to_string()],
        account_flat_confirmed_on: Some(day(1)),
        ..OwnerFile::default()
    });
    let summary = OrderSummary {
        client_order_id: "pt-a-1".to_string(),
        symbol: "SPY".to_string(),
        side: "buy".to_string(),
        qty: 10,
    };
    let ctx = AuthContext {
        strategy_id: "fake-strategy".to_string(),
        today: day(2),
        halted: false,
        risk_reducing: false,
        is_first_order: false,
        first_order_already_sent: false,
    };
    gate.authorize(&summary, &ctx).expect("permit")
}

fn order() -> NewOrder {
    NewOrder {
        client_order_id: "pt-a-1".to_string(),
        symbol: "SPY".to_string(),
        side: "buy".to_string(),
        qty: 10,
    }
}

// R-439: two 429 replies then 200 succeed, with three identical POSTs.
#[test]
fn order_post_is_sent_again_after_429() {
    let l = Listener::serve(vec![
        (429, String::new()),
        (429, String::new()),
        (200, REPLY.to_string()),
    ]);
    let fake = Fake::new();
    let http = client(&l.url, &fake);
    let result = submit_order(&http, permit(), &order());
    let seen = l.finish();
    assert!(result.is_ok(), "got {result:?}");
    assert_eq!(seen.len(), 3);
    for s in &seen {
        assert_eq!(s.method, "POST");
        assert_eq!(s.path(), "/v2/orders");
        assert_eq!(s.body, seen[0].body);
    }
}

// R-439: a 500 is not retried for an order, even with a 200 queued behind it.
#[test]
fn order_post_is_not_sent_again_after_500() {
    let l = Listener::serve(vec![(500, String::new()), (200, REPLY.to_string())]);
    let fake = Fake::new();
    let http = client(&l.url, &fake);
    let result = submit_order(&http, permit(), &order());
    // Release the listener thread, which still holds the unused 200 reply.
    let _ = http.get_trading("/v2/probe");
    let seen = l.finish();
    assert_eq!(result.unwrap_err(), AlpacaError::Server { status: 500 });
    assert_eq!(seen[0].method, "POST");
    assert_eq!(
        seen.iter().filter(|s| s.method == "POST").count(),
        1,
        "the order must be sent exactly once"
    );
}
