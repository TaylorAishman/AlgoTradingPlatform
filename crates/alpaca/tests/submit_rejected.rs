mod common;

use std::sync::Arc;
use std::time::Duration;

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
    let limiter = RateLimiter::new(200, mono, sleeper.clone());
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

fn permit_for(id: &str, symbol: &str, side: &str, qty: u64) -> OrderPermit {
    let gate = Gate::new(OwnerFile {
        stage: Stage::Paper,
        strategy_allowlist: vec!["fake-strategy".to_string()],
        account_flat_confirmed_on: Some(day(1)),
        ..OwnerFile::default()
    });
    let summary = OrderSummary {
        client_order_id: id.to_string(),
        symbol: symbol.to_string(),
        side: side.to_string(),
        qty,
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

fn new_order(id: &str, symbol: &str, side: &str, qty: u64) -> NewOrder {
    NewOrder {
        client_order_id: id.to_string(),
        symbol: symbol.to_string(),
        side: side.to_string(),
        qty,
    }
}

fn secs(v: &[u64]) -> Vec<Duration> {
    v.iter().map(|s| Duration::from_secs(*s)).collect()
}

fn submit(http: &HttpClient) -> Result<pt_alpaca::orders_read::WireOrder, AlpacaError> {
    submit_order(
        http,
        permit_for("pt-a-1", "SPY", "buy", 10),
        &new_order("pt-a-1", "SPY", "buy", 10),
    )
}

// R-101: a 403 is reported with the raw body after exactly one request and no sleep.
#[test]
fn rejection_is_sent_once_without_sleeping() {
    let body = r#"{"message":"insufficient buying power"}"#;
    let l = Listener::serve(vec![(403, body.to_string())]);
    let fake = Fake::new();
    let http = client(&l.url, &fake);
    let result = submit(&http);
    assert_eq!(fake.sleeps(), secs(&[]));
    let seen = l.finish();
    assert_eq!(seen.len(), 1);
    match result.unwrap_err() {
        AlpacaError::Rejected { status, message } => {
            assert_eq!(status, 403);
            assert!(message.contains("insufficient buying power"), "{message}");
        }
        other => panic!("got {other:?}"),
    }
}

// R-101: a 500 is never retried, because the broker may already hold the order.
#[test]
fn a_500_is_sent_exactly_once_and_never_retried() {
    let l = Listener::serve(vec![(500, String::new())]);
    let fake = Fake::new();
    let http = client(&l.url, &fake);
    let result = submit(&http);
    assert_eq!(fake.sleeps(), secs(&[]));
    assert_eq!(l.finish().len(), 1);
    assert_eq!(result.unwrap_err(), AlpacaError::Server { status: 500 });
}

// R-101: the same holds for a 503.
#[test]
fn a_503_is_sent_exactly_once_and_never_retried() {
    let l = Listener::serve(vec![(503, String::new())]);
    let fake = Fake::new();
    let http = client(&l.url, &fake);
    let result = submit(&http);
    assert_eq!(fake.sleeps(), secs(&[]));
    assert_eq!(l.finish().len(), 1);
    assert_eq!(result.unwrap_err(), AlpacaError::Server { status: 503 });
}

// R-085: 429, 429, 200 succeeds after sleeps of 15 s then 30 s, with three identical POSTs.
#[test]
fn two_429s_then_200_send_the_same_order_three_times() {
    let l = Listener::serve(vec![
        (429, String::new()),
        (429, String::new()),
        (200, REPLY.to_string()),
    ]);
    let fake = Fake::new();
    let http = client(&l.url, &fake);
    let result = submit(&http);
    assert_eq!(fake.sleeps(), secs(&[15, 30]));
    let seen = l.finish();
    assert_eq!(result.expect("order").client_order_id, "pt-a-1");
    assert_eq!(seen.len(), 3);
    assert!(seen.iter().all(|s| s.method == "POST"));
    assert_eq!(seen[0].body, seen[1].body);
    assert_eq!(seen[1].body, seen[2].body);
    assert!(seen[0].body.contains("pt-a-1"));
}

// R-085: one 429 then 200 is sent twice, after one sleep of 15 s.
#[test]
fn a_429_then_200_is_sent_twice() {
    let l = Listener::serve(vec![(429, String::new()), (200, REPLY.to_string())]);
    let fake = Fake::new();
    let http = client(&l.url, &fake);
    let result = submit(&http);
    assert_eq!(fake.sleeps(), secs(&[15]));
    assert_eq!(l.finish().len(), 2);
    assert!(result.is_ok());
}

// R-085: a permanent 429 is reported as RateLimited after 5 requests, sleeping 15, 30, 45, 60 s.
#[test]
fn permanent_429_is_reported_after_five_requests() {
    let l = Listener::serve((0..5).map(|_| (429, String::new())).collect());
    let fake = Fake::new();
    let http = client(&l.url, &fake);
    let result = submit(&http);
    assert_eq!(fake.sleeps(), secs(&[15, 30, 45, 60]));
    assert_eq!(l.finish().len(), 5);
    assert_eq!(result.unwrap_err(), AlpacaError::RateLimited);
}

// R-101 with R-085: a 429 then a 500 stops at the 500; the order is not sent a third time.
#[test]
fn a_500_after_a_429_is_not_retried() {
    let l = Listener::serve(vec![(429, String::new()), (500, String::new())]);
    let fake = Fake::new();
    let http = client(&l.url, &fake);
    let result = submit(&http);
    assert_eq!(fake.sleeps(), secs(&[15]));
    assert_eq!(l.finish().len(), 2);
    assert_eq!(result.unwrap_err(), AlpacaError::Server { status: 500 });
}
