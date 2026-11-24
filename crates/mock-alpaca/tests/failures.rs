//! R-113 (deny_feed) and R-116 (fail_next): the mock injects failures.

use pt_mock_alpaca::server::MockAlpaca;

fn status_of(mock: &MockAlpaca, path: &str) -> u16 {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .http_status_as_error(false)
        .build()
        .into();
    agent
        .get(&format!("{}{}", mock.url(), path))
        .header("APCA-API-KEY-ID", "key")
        .header("APCA-API-SECRET-KEY", "secret")
        .call()
        .expect("the request reaches the mock")
        .status()
        .as_u16()
}

fn bars_path(feed: &str) -> String {
    format!(
        "/v2/stocks/bars?symbols=SPY&timeframe=1Day&start=2024-01-02&end=2024-01-08&adjustment=raw&feed={feed}"
    )
}

fn seeded() -> MockAlpaca {
    let mock = MockAlpaca::start();
    for feed in ["sip", "iex"] {
        mock.seed_bars(
            "raw",
            feed,
            vec![("SPY".into(), "2024-01-02".into(), 1.0, 2.0, 0.5, 1.5, 10)],
        );
    }
    mock.seed_account("100000");
    mock
}

#[test]
fn denied_feed_is_answered_403() {
    // R-113
    let mock = seeded();
    assert_eq!(status_of(&mock, &bars_path("sip")), 200);
    mock.deny_feed("sip");
    assert_eq!(status_of(&mock, &bars_path("sip")), 403);
}

#[test]
fn denying_one_feed_leaves_other_feeds_served() {
    // R-113
    let mock = seeded();
    mock.deny_feed("sip");
    assert_eq!(status_of(&mock, &bars_path("iex")), 200);
}

#[test]
fn fail_next_two_answers_503_twice_then_serves_normally() {
    // R-116
    let mock = seeded();
    mock.fail_next(2, 503);
    assert_eq!(status_of(&mock, "/v2/account"), 503);
    assert_eq!(status_of(&mock, "/v2/clock"), 503);
    assert_eq!(status_of(&mock, "/v2/account"), 200);
    let reqs = mock.requests();
    assert_eq!(reqs.len(), 3, "all three requests are recorded");
    assert_eq!(reqs[0].path, "/v2/account");
    assert_eq!(reqs[1].path, "/v2/clock");
    assert_eq!(reqs[2].path, "/v2/account");
}

#[test]
fn fail_next_uses_the_given_status() {
    // R-116
    let mock = seeded();
    mock.fail_next(1, 500);
    assert_eq!(status_of(&mock, "/v2/account"), 500);
    assert_eq!(status_of(&mock, "/v2/account"), 200);
}
