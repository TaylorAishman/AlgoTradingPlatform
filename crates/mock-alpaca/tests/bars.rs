//! R-113: the mock serves seeded bars with filtering and pagination.

use pt_mock_alpaca::server::MockAlpaca;

const DATES: [&str; 5] = [
    "2024-01-02",
    "2024-01-03",
    "2024-01-04",
    "2024-01-05",
    "2024-01-08",
];

fn get_json(mock: &MockAlpaca, path: &str) -> serde_json::Value {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .http_status_as_error(false)
        .build()
        .into();
    let mut resp = agent
        .get(&format!("{}{}", mock.url(), path))
        .header("APCA-API-KEY-ID", "key")
        .header("APCA-API-SECRET-KEY", "secret")
        .call()
        .expect("the request reaches the mock");
    assert_eq!(resp.status().as_u16(), 200, "GET {path}");
    let text = resp.body_mut().read_to_string().expect("body is text");
    serde_json::from_str(&text).expect("body is JSON")
}

fn seeded() -> MockAlpaca {
    let mock = MockAlpaca::start();
    for symbol in ["SPY", "QQQ"] {
        let bars = DATES
            .iter()
            .enumerate()
            .map(|(i, d)| {
                let p = 100.0 + i as f64;
                (
                    symbol.to_string(),
                    d.to_string(),
                    p,
                    p + 1.0,
                    p - 1.0,
                    p + 0.5,
                    1000 + i as u64,
                )
            })
            .collect();
        mock.seed_bars("raw", "sip", bars);
    }
    mock
}

fn count(v: &serde_json::Value) -> usize {
    v["bars"]
        .as_object()
        .expect("bars is an object keyed by symbol")
        .values()
        .map(|a| a.as_array().expect("each symbol maps to an array").len())
        .sum()
}

fn base(extra: &str) -> String {
    format!(
        "/v2/stocks/bars?symbols=SPY,QQQ&timeframe=1Day&start=2024-01-02&end=2024-01-08&adjustment=raw&feed=sip{extra}"
    )
}

#[test]
fn bars_are_paged_four_four_two_with_a_null_final_token() {
    // R-113
    let mock = seeded();
    let p1 = get_json(&mock, &base("&limit=4"));
    assert_eq!(count(&p1), 4);
    let t1 = p1["next_page_token"]
        .as_str()
        .expect("first page has a token")
        .to_string();
    let p2 = get_json(&mock, &base(&format!("&limit=4&page_token={t1}")));
    assert_eq!(count(&p2), 4);
    let t2 = p2["next_page_token"]
        .as_str()
        .expect("second page has a token")
        .to_string();
    let p3 = get_json(&mock, &base(&format!("&limit=4&page_token={t2}")));
    assert_eq!(count(&p3), 2);
    assert!(p3["next_page_token"].is_null());
}

#[test]
fn every_bar_is_timestamped_at_0500z_of_its_date_and_carries_prices() {
    // R-113
    let mock = seeded();
    let v = get_json(&mock, &base("&limit=100"));
    assert_eq!(count(&v), 10);
    assert!(v["next_page_token"].is_null());
    for symbol in ["SPY", "QQQ"] {
        let arr = v["bars"][symbol].as_array().expect("symbol present");
        let ts: Vec<&str> = arr.iter().map(|b| b["t"].as_str().unwrap()).collect();
        let want: Vec<String> = DATES.iter().map(|d| format!("{d}T05:00:00Z")).collect();
        assert_eq!(ts, want);
        assert!(arr[0]["o"].is_number() && arr[0]["c"].is_number() && arr[0]["v"].is_number());
    }
}

#[test]
fn adjustment_all_returns_no_bars_when_only_raw_is_seeded() {
    // R-113
    let mock = seeded();
    let v = get_json(
        &mock,
        "/v2/stocks/bars?symbols=SPY,QQQ&timeframe=1Day&start=2024-01-02&end=2024-01-08&adjustment=all&feed=sip&limit=4",
    );
    assert_eq!(count(&v), 0);
}

#[test]
fn other_feed_returns_no_bars() {
    // R-113
    let mock = seeded();
    let v = get_json(
        &mock,
        "/v2/stocks/bars?symbols=SPY&timeframe=1Day&start=2024-01-02&end=2024-01-08&adjustment=raw&feed=iex",
    );
    assert_eq!(count(&v), 0);
}

#[test]
fn only_requested_symbols_and_dates_are_returned() {
    // R-113
    let mock = seeded();
    let v = get_json(
        &mock,
        "/v2/stocks/bars?symbols=SPY&timeframe=1Day&start=2024-01-03&end=2024-01-04&adjustment=raw&feed=sip",
    );
    assert_eq!(count(&v), 2);
    let arr = v["bars"]["SPY"].as_array().expect("SPY present");
    assert_eq!(arr[0]["t"], "2024-01-03T05:00:00Z");
    assert_eq!(arr[1]["t"], "2024-01-04T05:00:00Z");
}
