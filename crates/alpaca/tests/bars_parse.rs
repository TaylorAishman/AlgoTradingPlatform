mod common;

use std::str::FromStr;
use std::sync::Arc;

use chrono::NaiveDate;
use pt_alpaca::auth::Credentials;
use pt_alpaca::data::{daily_bars, WireBar};
use pt_alpaca::endpoints::Endpoints;
use pt_alpaca::http::HttpClient;
use pt_alpaca::limiter::{MonoClock, RateLimiter, Sleeper};
use rust_decimal::Decimal;

use common::{Fake, Listener};

/// Nothing listens here, so a request sent to the trading host fails at once.
const DEAD_TRADING: &str = "http://127.0.0.1:1";

const FIXTURE: &str = include_str!("fixtures/bars.json");

fn client(trading: &str, data: &str) -> HttpClient {
    let fake = Fake::new();
    let mono: Arc<dyn MonoClock> = fake.clone();
    let sleeper: Arc<dyn Sleeper> = fake.clone();
    let limiter = RateLimiter::new(200, mono, sleeper.clone()).expect("a limit of 200 is accepted");
    let endpoints = Endpoints::with_overrides(Some(trading), Some(data)).expect("loopback");
    HttpClient::new(
        endpoints,
        Credentials::new("FAKEKEYID".to_string(), "FAKESECRET".to_string()),
        limiter,
        sleeper,
    )
}

fn dec(s: &str) -> Decimal {
    Decimal::from_str(s).expect("decimal")
}

fn fetch(body: &str, symbols: &[&str]) -> Vec<WireBar> {
    let l = Listener::serve(vec![(200, body.to_string())]);
    let http = client(DEAD_TRADING, &l.url);
    let symbols: Vec<String> = symbols.iter().map(|s| s.to_string()).collect();
    let start = NaiveDate::from_ymd_opt(2024, 1, 2).unwrap();
    let end = NaiveDate::from_ymd_opt(2024, 1, 5).unwrap();
    let result = daily_bars(&http, &symbols, start, end, "all", "sip");
    let bars = result.expect("daily_bars should return Ok");
    let seen = l.finish();
    assert_eq!(seen.len(), 1);
    bars
}

// R-089: the documented response parses to one WireBar with exact decimals.
#[test]
fn documented_response_parses_to_one_wire_bar() {
    let bars = fetch(FIXTURE, &["SPY"]);
    assert_eq!(bars.len(), 1, "{bars:?}");
    let bar = &bars[0];
    assert_eq!(bar.symbol, "SPY");
    assert_eq!(bar.date, NaiveDate::from_ymd_opt(2024, 1, 2).unwrap());
    assert_eq!(bar.open, dec("472.16"));
    assert_eq!(bar.high, dec("473.67"));
    assert_eq!(bar.low, dec("470.49"));
    assert_eq!(bar.close, dec("472.65"));
    assert_eq!(bar.volume, 123623511);
}

// R-089: prices are the shortest decimal text of the number, with no float noise.
#[test]
fn prices_carry_no_float_noise() {
    let body = r#"{"bars":{"SPY":[{"t":"2024-01-02T05:00:00Z","o":0.1,"h":0.30000000000000004,"l":1e-2,"c":100,"v":7}]},"next_page_token":null}"#;
    let bars = fetch(body, &["SPY"]);
    assert_eq!(bars.len(), 1, "{bars:?}");
    assert_eq!(bars[0].open.to_string(), "0.1");
    assert_eq!(bars[0].high.to_string(), "0.30000000000000004");
    assert_eq!(bars[0].low, dec("0.01"));
    assert_eq!(bars[0].close, dec("100"));
    assert_eq!(bars[0].volume, 7);
}

// R-089: every symbol's bars are returned, tagged with their symbol, in served order.
#[test]
fn bars_of_several_symbols_keep_their_symbol() {
    let body = r#"{"bars":{
        "SPY":[{"t":"2024-01-02T05:00:00Z","o":1.5,"h":2.5,"l":1.25,"c":2.0,"v":10},
               {"t":"2024-01-03T05:00:00Z","o":2.0,"h":3.0,"l":1.75,"c":2.75,"v":11}],
        "QQQ":[{"t":"2024-01-02T05:00:00Z","o":400.1,"h":401.2,"l":399.3,"c":400.4,"v":12}]
    },"next_page_token":null}"#;
    let bars = fetch(body, &["SPY", "QQQ"]);
    assert_eq!(bars.len(), 3, "{bars:?}");
    let spy: Vec<&WireBar> = bars.iter().filter(|b| b.symbol == "SPY").collect();
    let qqq: Vec<&WireBar> = bars.iter().filter(|b| b.symbol == "QQQ").collect();
    assert_eq!(spy.len(), 2);
    assert_eq!(qqq.len(), 1);
    assert_eq!(spy[0].date, NaiveDate::from_ymd_opt(2024, 1, 2).unwrap());
    assert_eq!(spy[1].date, NaiveDate::from_ymd_opt(2024, 1, 3).unwrap());
    assert_eq!(spy[1].close, dec("2.75"));
    assert_eq!(spy[1].volume, 11);
    assert_eq!(qqq[0].open, dec("400.1"));
    assert_eq!(qqq[0].volume, 12);
}

// R-089: a response with no bars gives an empty list, not an error.
#[test]
fn empty_bars_object_gives_no_bars() {
    let bars = fetch(r#"{"bars":{},"next_page_token":null}"#, &["SPY"]);
    assert!(bars.is_empty(), "{bars:?}");
}
