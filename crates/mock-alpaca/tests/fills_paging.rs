//! R-115, R-099: the FILL activities route pages the way the client reads it.

use pt_mock_alpaca::server::MockAlpaca;

const EPOCH: &str = "2000-01-01T00:00:00Z";

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .http_status_as_error(false)
        .build()
        .into()
}

/// GET the FILL activities with `query`; returns the status and the body.
fn fills(mock: &MockAlpaca, query: &str) -> (u16, serde_json::Value) {
    let mut resp = agent()
        .get(&format!("{}/v2/account/activities/FILL{query}", mock.url()))
        .header("APCA-API-KEY-ID", "key")
        .header("APCA-API-SECRET-KEY", "secret")
        .call()
        .expect("the request reaches the mock");
    let status = resp.status().as_u16();
    let text = resp.body_mut().read_to_string().expect("body reads");
    (status, serde_json::from_str(&text).expect("json body"))
}

fn ids(v: &serde_json::Value) -> Vec<String> {
    v.as_array()
        .expect("a JSON array")
        .iter()
        .map(|a| a["id"].as_str().expect("an id").to_string())
        .collect()
}

/// A mock with one buy order of `n` SPY, filled one share at a time.
fn with_fills(n: u64) -> MockAlpaca {
    let mock = MockAlpaca::start();
    mock.seed_account("1000000");
    let body = serde_json::json!({
        "symbol": "SPY", "qty": n.to_string(), "side": "buy", "type": "market",
        "time_in_force": "day", "client_order_id": "c1",
    });
    let resp = agent()
        .post(&format!("{}/v2/orders", mock.url()))
        .header("APCA-API-KEY-ID", "key")
        .header("APCA-API-SECRET-KEY", "secret")
        .header("Content-Type", "application/json")
        .send(body.to_string())
        .expect("the request reaches the mock");
    assert_eq!(resp.status().as_u16(), 200);
    for _ in 0..n {
        mock.fill("c1", 1, "100");
    }
    mock
}

/// Read every fill after `after` the way `pt_alpaca::activities::fill_activities`
/// does: ask again with `page_token` set to the last id while a page is full.
/// Returns the ids in the order served and the size of each page.
fn read_all(mock: &MockAlpaca, page_size: usize, after: &str) -> (Vec<String>, Vec<usize>) {
    let base = format!("?direction=asc&page_size={page_size}&after={after}");
    let (mut all, mut pages, mut token) = (Vec::new(), Vec::new(), None::<String>);
    loop {
        assert!(pages.len() < 50, "paging did not end");
        let query = match &token {
            Some(t) => format!("{base}&page_token={t}"),
            None => base.clone(),
        };
        let (status, body) = fills(mock, &query);
        assert_eq!(status, 200);
        let page = ids(&body);
        pages.push(page.len());
        if let Some(last) = page.last() {
            token = Some(last.clone());
        }
        let full = page.len() >= page_size;
        all.extend(page);
        if !full {
            return (all, pages);
        }
    }
}

#[test]
fn the_client_loop_returns_every_fill_exactly_once() {
    // R-099: pages of 100, as the client asks for them.
    let mock = with_fills(150);
    let (all, pages) = read_all(&mock, 100, EPOCH);
    assert_eq!(pages, vec![100, 50]);
    assert_eq!(all[..100], ids(&fills(&mock, "").1)[..]);
    let mut unique = all.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(unique.len(), 150);
}

#[test]
fn a_last_page_that_is_exactly_full_is_followed_by_an_empty_one() {
    let mock = with_fills(4);
    let (all, pages) = read_all(&mock, 2, EPOCH);
    assert_eq!(pages, vec![2, 2, 0]);
    assert_eq!(all.len(), 4);
}

#[test]
fn page_size_defaults_to_100_and_is_capped_at_100() {
    let mock = with_fills(101);
    assert_eq!(ids(&fills(&mock, "").1).len(), 100);
    assert_eq!(ids(&fills(&mock, "?page_size=500").1).len(), 100);
}

#[test]
fn after_keeps_only_later_fills() {
    let mock = with_fills(5);
    let (_, everything) = fills(&mock, "");
    let second = everything[1]["transaction_time"].as_str().expect("a time");
    let (all, _) = read_all(&mock, 100, second);
    assert_eq!(all, ids(&everything)[2..].to_vec());
}

#[test]
fn direction_desc_serves_newest_first_and_pages_backwards() {
    let mock = with_fills(3);
    let oldest_first = ids(&fills(&mock, "").1);
    assert_eq!(ids(&fills(&mock, "?direction=asc").1), oldest_first);
    let newest_first = ids(&fills(&mock, "?direction=desc").1);
    assert_eq!(
        newest_first,
        oldest_first.iter().rev().cloned().collect::<Vec<_>>()
    );
    let (_, page) = fills(
        &mock,
        &format!("?direction=desc&page_token={}", oldest_first[1]),
    );
    assert_eq!(ids(&page), vec![oldest_first[0].clone()]);
}

#[test]
fn an_unknown_page_token_is_refused() {
    let mock = with_fills(2);
    let (status, _) = fills(&mock, "?direction=asc&page_token=nope");
    assert_eq!(status, 422);
}

#[test]
fn a_token_naming_the_newest_fill_gives_an_empty_page() {
    // R-115: the reply that ends the client's loop after an exactly full page.
    let mock = with_fills(3);
    let newest = ids(&fills(&mock, "").1)[2].clone();
    let (status, body) = fills(&mock, &format!("?direction=asc&page_token={newest}"));
    assert_eq!(status, 200);
    assert_eq!(body, serde_json::json!([]));
}
