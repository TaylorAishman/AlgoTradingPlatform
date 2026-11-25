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

// R-100: a matching permit sends one POST to /v2/orders with the exact six-key body.
#[test]
fn matching_permit_sends_one_post_with_the_exact_body() {
    let l = Listener::serve(vec![(200, REPLY.to_string())]);
    let fake = Fake::new();
    let http = client(&l.url, &fake);
    let result = submit_order(
        &http,
        permit_for("pt-a-1", "SPY", "buy", 10),
        &new_order("pt-a-1", "SPY", "buy", 10),
    );
    let seen = l.finish();
    assert!(result.is_ok(), "got {result:?}");
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].method, "POST");
    assert_eq!(seen[0].path(), "/v2/orders");
    let body: serde_json::Value = serde_json::from_str(&seen[0].body).expect("json body");
    let want = serde_json::json!({
        "symbol": "SPY",
        "qty": "10",
        "side": "buy",
        "type": "market",
        "time_in_force": "day",
        "client_order_id": "pt-a-1"
    });
    assert_eq!(body, want);
}

// R-100: the reply is parsed into the returned order.
#[test]
fn matching_permit_returns_the_parsed_order() {
    let l = Listener::serve(vec![(200, REPLY.to_string())]);
    let fake = Fake::new();
    let http = client(&l.url, &fake);
    let order = submit_order(
        &http,
        permit_for("pt-a-1", "SPY", "buy", 10),
        &new_order("pt-a-1", "SPY", "buy", 10),
    )
    .expect("order");
    l.finish();
    assert_eq!(order.id, "b9");
    assert_eq!(order.client_order_id, "pt-a-1");
    assert_eq!(order.symbol, "SPY");
    assert_eq!(order.qty, 10);
    assert_eq!(order.filled_qty, 0);
    assert_eq!(order.status, "accepted");
}

// R-082: the POST carries both auth headers and a JSON content type.
#[test]
fn post_carries_auth_and_content_type_headers() {
    let l = Listener::serve(vec![(200, REPLY.to_string())]);
    let fake = Fake::new();
    let http = client(&l.url, &fake);
    submit_order(
        &http,
        permit_for("pt-a-1", "SPY", "buy", 10),
        &new_order("pt-a-1", "SPY", "buy", 10),
    )
    .expect("order");
    let seen = l.finish();
    assert_eq!(seen[0].header("APCA-API-KEY-ID"), Some("FAKEKEYID"));
    assert_eq!(seen[0].header("APCA-API-SECRET-KEY"), Some("FAKESECRET"));
    let ct = seen[0].header("Content-Type").expect("content type");
    assert!(ct.starts_with("application/json"), "got {ct}");
}

// R-100: every field the permit is bound to is checked, and nothing is sent on a mismatch.
// The listener has one reply it must never use; it is released by a probe request after.
fn assert_refused(permit: OrderPermit, order: &NewOrder) {
    let l = Listener::serve(vec![(200, REPLY.to_string())]);
    let fake = Fake::new();
    let http = client(&l.url, &fake);
    let result = submit_order(&http, permit, order);
    assert_eq!(result.unwrap_err(), AlpacaError::PermitMismatch);
    assert!(fake.sleeps().is_empty(), "no limiter slot may be taken");
    // Release the listener thread with a plain request, then check it was the only one.
    let _ = http.get_trading("/v2/probe");
    let seen = l.finish();
    assert_eq!(seen.len(), 1, "the refused order must not have been sent");
    assert_eq!(seen[0].method, "GET");
    assert_eq!(seen[0].path(), "/v2/probe");
}

#[test]
fn quantity_mismatch_is_refused_before_any_request() {
    assert_refused(
        permit_for("pt-a-1", "SPY", "buy", 10),
        &new_order("pt-a-1", "SPY", "buy", 11),
    );
}

#[test]
fn symbol_mismatch_is_refused_before_any_request() {
    assert_refused(
        permit_for("pt-a-1", "SPY", "buy", 10),
        &new_order("pt-a-1", "QQQ", "buy", 10),
    );
}

#[test]
fn side_mismatch_is_refused_before_any_request() {
    assert_refused(
        permit_for("pt-a-1", "SPY", "buy", 10),
        &new_order("pt-a-1", "SPY", "sell", 10),
    );
}

#[test]
fn client_order_id_mismatch_is_refused_before_any_request() {
    assert_refused(
        permit_for("pt-a-1", "SPY", "buy", 10),
        &new_order("pt-a-2", "SPY", "buy", 10),
    );
}

// R-082: a 403 is reported as Rejected with the raw body, after one request.
#[test]
fn rejection_carries_status_and_raw_body() {
    let body = r#"{"message":"insufficient buying power"}"#;
    let l = Listener::serve(vec![(403, body.to_string())]);
    let fake = Fake::new();
    let http = client(&l.url, &fake);
    let result = submit_order(
        &http,
        permit_for("pt-a-1", "SPY", "buy", 10),
        &new_order("pt-a-1", "SPY", "buy", 10),
    );
    let seen = l.finish();
    assert_eq!(seen.len(), 1);
    assert_eq!(
        result.unwrap_err(),
        AlpacaError::Rejected {
            status: 403,
            message: body.to_string()
        }
    );
}
