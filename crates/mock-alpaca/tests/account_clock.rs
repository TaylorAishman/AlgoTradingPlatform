//! R-112: the mock serves the seeded account and clock.

use pt_mock_alpaca::server::MockAlpaca;

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

#[test]
fn seeded_account_reports_cash_equity_and_status() {
    // R-112
    let mock = MockAlpaca::start();
    mock.seed_account("100000");
    let v = get_json(&mock, "/v2/account");
    assert_eq!(v["cash"], "100000");
    assert_eq!(v["equity"], "100000");
    assert_eq!(v["status"], "ACTIVE");
    assert_eq!(v["multiplier"], "2");
    assert_eq!(v["shorting_enabled"], true);
}

#[test]
fn account_follows_the_seeded_cash() {
    // R-112
    let mock = MockAlpaca::start();
    mock.seed_account("2500");
    let v = get_json(&mock, "/v2/account");
    assert_eq!(v["cash"], "2500");
    assert_eq!(v["equity"], "2500");
}

#[test]
fn set_clock_is_reported_by_the_clock_route() {
    // R-112
    let mock = MockAlpaca::start();
    mock.set_clock(
        "2024-01-02T10:00:00-05:00",
        false,
        "2024-01-03T09:30:00-05:00",
        "2024-01-03T16:00:00-05:00",
    );
    let v = get_json(&mock, "/v2/clock");
    assert_eq!(v["timestamp"], "2024-01-02T10:00:00-05:00");
    assert_eq!(v["is_open"], false);
    assert_eq!(v["next_open"], "2024-01-03T09:30:00-05:00");
    assert_eq!(v["next_close"], "2024-01-03T16:00:00-05:00");
}

#[test]
fn set_clock_open_flag_true_is_reported() {
    // R-112
    let mock = MockAlpaca::start();
    mock.set_clock(
        "2024-01-02T10:00:00-05:00",
        true,
        "2024-01-03T09:30:00-05:00",
        "2024-01-02T16:00:00-05:00",
    );
    let v = get_json(&mock, "/v2/clock");
    assert_eq!(v["is_open"], true);
}

#[test]
fn account_has_all_nine_fields_with_the_documented_types() {
    // R-112, R-093: architecture 5.9 lists nine account fields
    let mock = MockAlpaca::start();
    mock.seed_account("100000");
    let v = get_json(&mock, "/v2/account");
    for k in [
        "status",
        "cash",
        "equity",
        "last_equity",
        "buying_power",
        "multiplier",
    ] {
        assert!(v[k].is_string(), "{k} is a string");
    }
    for k in ["shorting_enabled", "trading_blocked", "account_blocked"] {
        assert!(v[k].is_boolean(), "{k} is a boolean");
    }
    assert_eq!(v["last_equity"], "100000");
    assert_eq!(v["buying_power"], "200000");
    assert_eq!(v["trading_blocked"], false);
    assert_eq!(v["account_blocked"], false);
}

#[test]
fn buying_power_is_cash_times_the_multiplier_for_fractional_cash() {
    // R-112
    for (cash, expected) in [("2500.55", "5001.10"), ("0.25", "0.50"), ("0.04", "0.08")] {
        let mock = MockAlpaca::start();
        mock.seed_account(cash);
        assert_eq!(get_json(&mock, "/v2/account")["buying_power"], expected);
    }
}
