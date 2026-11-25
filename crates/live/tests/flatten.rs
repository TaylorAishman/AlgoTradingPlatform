//! R-322: the owner can flatten the account, even while halted, and the system stays halted.

mod common;

use common::{d, sym, LiveFixture};
use pt_alpaca::http::loopback_client;
use pt_alpaca::orders_read::order_by_client_id;
use pt_core::order_state::OrderState;
use pt_core::types::Side;
use pt_gate::owner::Stage;
use pt_live::context::LiveContext;
use pt_live::killswitch::flatten;
use pt_live::reconcile::reconcile;
use pt_store::trading::{Book, LedgerEntry, OrderRow};
use rust_decimal::Decimal;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};

const SPY_ID: &str = "pt-flatten-20240301-SPY";
const TLT_ID: &str = "pt-flatten-20240301-TLT";

fn fill(symbol: &str, qty: i64) -> LedgerEntry {
    LedgerEntry {
        book: Book::Paper,
        date: d(2024, 3, 1),
        kind: "fill".to_string(),
        symbol: Some(sym(symbol)),
        qty,
        price: None,
        cash_delta: Decimal::ZERO,
        ref_id: None,
    }
}

/// Ledger and mock positions SPY +10 and TLT -5, reconciled, with an active `manual` halt.
fn halted_fixture() -> LiveFixture {
    let fx = LiveFixture::new(Stage::Paper);
    assert!(reconcile(&fx.ctx()).expect("opening").clean);
    fx.store
        .append_ledger(&fill("SPY", 10))
        .expect("ledger SPY");
    fx.store
        .append_ledger(&fill("TLT", -5))
        .expect("ledger TLT");
    fx.mock.seed_position("SPY", 10);
    fx.mock.seed_position("TLT", -5);
    assert!(reconcile(&fx.ctx()).expect("reconcile").clean);
    fx.store
        .set_halt("manual", "halted for the test", fx.now())
        .expect("halt");
    fx
}

fn order_posts(fx: &LiveFixture) -> Vec<serde_json::Value> {
    let mut bodies: Vec<serde_json::Value> = fx
        .mock
        .requests()
        .into_iter()
        .filter(|r| r.method == "POST" && r.path == "/v2/orders")
        .map(|r| serde_json::from_str(&r.body).expect("json body"))
        .collect();
    bodies.sort_by_key(|b| b["symbol"].as_str().unwrap_or_default().to_string());
    bodies
}

fn qty_of(body: &serde_json::Value) -> String {
    body["qty"].to_string().trim_matches('"').to_string()
}

// R-322
#[test]
fn flatten_while_halted_closes_both_positions_and_returns_two() {
    let fx = halted_fixture();

    let n = flatten(&fx.ctx()).expect("flatten while halted");

    assert_eq!(n, 2);
    let posts = order_posts(&fx);
    assert_eq!(posts.len(), 2, "two POSTs");
    assert_eq!(posts[0]["symbol"], "SPY");
    assert_eq!(posts[0]["side"], "sell");
    assert_eq!(qty_of(&posts[0]), "10");
    assert_eq!(posts[1]["symbol"], "TLT");
    assert_eq!(posts[1]["side"], "buy");
    assert_eq!(qty_of(&posts[1]), "5");
}

// R-322
#[test]
fn flatten_orders_carry_the_flatten_client_order_id_prefix() {
    let fx = halted_fixture();

    flatten(&fx.ctx()).expect("flatten");

    let posts = order_posts(&fx);
    assert_eq!(posts.len(), 2);
    for p in &posts {
        let id = p["client_order_id"].as_str().expect("client_order_id");
        assert!(id.starts_with("pt-flatten-"), "id was {id:?}");
    }
}

// R-322
#[test]
fn after_flatten_the_store_has_an_active_halt_with_reason_flattened() {
    let fx = halted_fixture();

    flatten(&fx.ctx()).expect("flatten");

    let halts = fx.store.active_halts().expect("halts");
    assert!(
        halts.iter().any(|h| h.reason == "flattened"),
        "halts were {halts:?}"
    );
}

/// Starts a loopback proxy in front of `upstream` that answers the first
/// `POST /v2/orders` with 403, as a broker refusing that order would, and
/// forwards every other request unchanged. Returns the proxy's base URL.
fn reject_first_order_proxy(upstream: &str) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind proxy");
    let addr = listener.local_addr().expect("proxy addr");
    let upstream = upstream.trim_start_matches("http://").to_string();
    std::thread::spawn(move || {
        let mut rejected = false;
        for stream in listener.incoming() {
            let Ok(client) = stream else { break };
            let mut reader = BufReader::new(client);
            let mut head = Vec::new();
            let mut length = 0usize;
            loop {
                let mut line = String::new();
                if reader.read_line(&mut line).unwrap_or(0) == 0 {
                    break;
                }
                let trimmed = line.trim_end();
                if trimmed.is_empty() {
                    break;
                }
                let lower = trimmed.to_ascii_lowercase();
                if let Some(v) = lower.strip_prefix("content-length:") {
                    length = v.trim().parse().unwrap_or(0);
                }
                if !lower.starts_with("connection:") {
                    head.push(trimmed.to_string());
                }
            }
            let mut body = vec![0u8; length];
            if reader.read_exact(&mut body).is_err() {
                continue;
            }
            let mut client = reader.into_inner();
            let request_line = head.first().cloned().unwrap_or_default();
            if !rejected && request_line.starts_with("POST /v2/orders ") {
                rejected = true;
                let msg = r#"{"message":"injected rejection"}"#;
                let _ = write!(
                    client,
                    "HTTP/1.1 403 Forbidden\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{msg}",
                    msg.len()
                );
                continue;
            }
            let mut request = head.join("\r\n").into_bytes();
            request.extend_from_slice(b"\r\nConnection: close\r\n\r\n");
            request.extend_from_slice(&body);
            let mut up = TcpStream::connect(&upstream).expect("connect mock");
            up.write_all(&request).expect("forward request");
            let mut reply = Vec::new();
            let _ = up.read_to_end(&mut reply);
            let _ = client.write_all(&reply);
        }
    });
    format!("http://{addr}")
}

// R-322
#[test]
fn a_broker_rejection_marks_that_order_rejected_alerts_and_still_closes_the_rest() {
    let fx = halted_fixture();
    let proxy = loopback_client(&reject_first_order_proxy(&fx.mock.url())).expect("proxy client");
    let ctx = LiveContext {
        http: &proxy,
        ..fx.ctx()
    };

    let n = flatten(&ctx).expect("a rejection does not abort the flatten");

    assert_eq!(n, 1, "only the accepted order counts as sent");
    let posts = order_posts(&fx);
    assert_eq!(posts.len(), 1, "the refused POST never reached the broker");
    assert_eq!(posts[0]["symbol"], "TLT");
    let spy = fx.store.order(SPY_ID).expect("read").expect("SPY row");
    assert_eq!(spy.state, OrderState::Rejected);
    let tlt = fx.store.order(TLT_ID).expect("read").expect("TLT row");
    assert_eq!(tlt.state, OrderState::Submitted);
    assert!(tlt.broker_order_id.is_some());
    let alerts = fx.store.alerts().expect("alerts");
    let rejected: Vec<_> = alerts
        .iter()
        .filter(|(_, code, _, _)| code == "order_rejected")
        .collect();
    assert_eq!(rejected.len(), 1, "alerts: {alerts:?}");
    assert!(rejected[0].2.contains("SPY"), "message: {}", rejected[0].2);
}

// R-322
#[test]
fn a_repeat_flatten_adopts_an_order_the_broker_already_holds_without_posting_it() {
    let fx = halted_fixture();
    fx.store
        .insert_order(&OrderRow {
            client_order_id: SPY_ID.to_string(),
            decision_id: None,
            symbol: sym("SPY"),
            side: Side::Sell,
            qty: 10,
            leg: 0,
            state: OrderState::Intent,
            broker_order_id: None,
            filled_qty: 0,
            reference_price: Decimal::ZERO,
            trade_date: d(2024, 3, 1),
        })
        .expect("insert intent");
    fx.mock.seed_foreign_order(SPY_ID, "SPY", "sell", 10);
    let broker_id = order_by_client_id(&fx.http, SPY_ID)
        .expect("lookup")
        .expect("broker holds the order")
        .id;

    let n = flatten(&fx.ctx()).expect("repeat flatten");

    assert_eq!(n, 1, "only TLT is sent");
    let posts = order_posts(&fx);
    assert_eq!(posts.len(), 1);
    assert_eq!(posts[0]["symbol"], "TLT");
    let spy = fx.store.order(SPY_ID).expect("read").expect("SPY row");
    assert_eq!(spy.state, OrderState::Submitted);
    assert_eq!(spy.broker_order_id.as_deref(), Some(broker_id.as_str()));
}

// R-322
#[test]
fn a_retry_after_a_rejection_sends_one_fresh_order_and_keeps_one_flattened_halt() {
    let fx = halted_fixture();
    let proxy = loopback_client(&reject_first_order_proxy(&fx.mock.url())).expect("proxy client");
    let ctx = LiveContext {
        http: &proxy,
        ..fx.ctx()
    };
    flatten(&ctx).expect("first flatten");
    let before = order_posts(&fx).len();

    let n = flatten(&fx.ctx()).expect("retry after a rejection");

    assert_eq!(n, 1, "only the rejected SPY is sent again");
    let posts = order_posts(&fx);
    assert_eq!(posts.len(), before + 1, "exactly one extra POST");
    let retry_id = format!("{SPY_ID}-r1");
    assert!(posts
        .iter()
        .any(|p| p["client_order_id"] == retry_id.as_str()));
    let spy = fx.store.order(&retry_id).expect("read").expect("retry row");
    assert_eq!(spy.state, OrderState::Submitted);
    assert!(spy.broker_order_id.is_some());
    let halts = fx.store.active_halts().expect("halts");
    assert_eq!(halts.iter().filter(|h| h.reason == "flattened").count(), 1);
}
