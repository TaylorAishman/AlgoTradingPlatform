//! R-112: the mock serves the seeded calendar and assets.

use pt_mock_alpaca::server::MockAlpaca;

fn get(mock: &MockAlpaca, path: &str) -> (u16, String) {
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
    let status = resp.status().as_u16();
    let text = resp.body_mut().read_to_string().expect("body is text");
    (status, text)
}

fn get_json(mock: &MockAlpaca, path: &str) -> serde_json::Value {
    let (status, text) = get(mock, path);
    assert_eq!(status, 200, "GET {path}");
    serde_json::from_str(&text).expect("body is JSON")
}

#[test]
fn calendar_returns_only_sessions_inside_the_range() {
    // R-112
    let mock = MockAlpaca::start();
    mock.seed_sessions(&["2024-01-02", "2024-01-03"]);
    let v = get_json(&mock, "/v2/calendar?start=2024-01-03&end=2024-12-31");
    let entries = v.as_array().expect("calendar is a JSON array");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0]["date"], "2024-01-03");
}

#[test]
fn calendar_returns_every_session_in_a_wide_range() {
    // R-112
    let mock = MockAlpaca::start();
    mock.seed_sessions(&["2024-01-02", "2024-01-03"]);
    let v = get_json(&mock, "/v2/calendar?start=2024-01-01&end=2024-12-31");
    let dates: Vec<&str> = v
        .as_array()
        .expect("calendar is a JSON array")
        .iter()
        .map(|e| e["date"].as_str().expect("date is a string"))
        .collect();
    assert_eq!(dates, vec!["2024-01-02", "2024-01-03"]);
}

#[test]
fn calendar_excludes_sessions_after_the_end() {
    // R-112
    let mock = MockAlpaca::start();
    mock.seed_sessions(&["2024-01-02", "2024-01-03"]);
    let v = get_json(&mock, "/v2/calendar?start=2024-01-01&end=2024-01-02");
    let entries = v.as_array().expect("calendar is a JSON array");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0]["date"], "2024-01-02");
}

#[test]
fn seeded_asset_reports_tradable_shortable_and_borrow_status() {
    // R-112
    let mock = MockAlpaca::start();
    mock.seed_asset("SPY", true, true, true);
    let v = get_json(&mock, "/v2/assets/SPY");
    assert_eq!(v["tradable"], true);
    assert_eq!(v["shortable"], true);
    assert_eq!(v["borrow_status"], "easy_to_borrow");
}

#[test]
fn asset_flags_follow_the_seed() {
    // R-112
    let mock = MockAlpaca::start();
    mock.seed_asset("XYZ", true, false, false);
    let v = get_json(&mock, "/v2/assets/XYZ");
    assert_eq!(v["tradable"], true);
    assert_eq!(v["shortable"], false);
    assert_ne!(v["borrow_status"], "easy_to_borrow");
}

#[test]
fn unknown_asset_is_404() {
    // R-112
    let mock = MockAlpaca::start();
    mock.seed_asset("SPY", true, true, true);
    let (status, _) = get(&mock, "/v2/assets/NOPE");
    assert_eq!(status, 404);
}
