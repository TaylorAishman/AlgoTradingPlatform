//! Route handling: maps a recorded request to a status and JSON body.

use serde_json::json;

use crate::state::{Recorded, State};

/// Double a non-negative decimal string exactly (buying power is cash times the
/// fixed multiplier of 2). Falls back to the input if it is not a plain decimal.
fn times_two(cash: &str) -> String {
    let (int, frac) = cash.split_once('.').unwrap_or((cash, ""));
    let digits = format!("{int}{frac}");
    match digits.parse::<u128>() {
        Ok(n) if !digits.is_empty() => {
            let mut d = (n * 2).to_string();
            if d.len() <= frac.len() {
                d = format!("{}{d}", "0".repeat(frac.len() + 1 - d.len()));
            }
            let at = d.len() - frac.len();
            if frac.is_empty() {
                d
            } else {
                format!("{}.{}", &d[..at], &d[at..])
            }
        }
        _ => cash.to_string(),
    }
}

/// Answer a request from the seeded state; anything unseeded is 404.
pub fn respond(req: &Recorded, state: &State) -> (u16, String) {
    if req.method == "GET" {
        match req.path.as_str() {
            "/v2/account" => {
                if let Some(cash) = &state.account_cash {
                    let body = json!({
                        "cash": cash,
                        "equity": cash,
                        "last_equity": cash,
                        "buying_power": times_two(cash),
                        "status": "ACTIVE",
                        "multiplier": "2",
                        "shorting_enabled": true,
                        "trading_blocked": false,
                        "account_blocked": false,
                    });
                    return (200, body.to_string());
                }
            }
            "/v2/clock" => {
                if let Some(c) = &state.clock {
                    let body = json!({
                        "timestamp": c.timestamp,
                        "is_open": c.is_open,
                        "next_open": c.next_open,
                        "next_close": c.next_close,
                    });
                    return (200, body.to_string());
                }
            }
            "/v2/calendar" => {
                let param = |k: &str| {
                    req.query
                        .split('&')
                        .filter_map(|kv| kv.split_once('='))
                        .find(|(name, _)| *name == k)
                        .map(|(_, v)| v.to_string())
                };
                let start = param("start");
                let end = param("end");
                let entries: Vec<_> = state
                    .sessions
                    .iter()
                    .filter(|d| start.as_ref().is_none_or(|s| d.as_str() >= s.as_str()))
                    .filter(|d| end.as_ref().is_none_or(|e| d.as_str() <= e.as_str()))
                    .map(|d| json!({"date": d, "open": "09:30", "close": "16:00"}))
                    .collect();
                return (200, serde_json::Value::Array(entries).to_string());
            }
            "/v2/stocks/bars" => return bars(&req.query, state),
            path => {
                if let Some(sym) = path.strip_prefix("/v2/assets/") {
                    if let Some(a) = state.assets.iter().find(|a| a.symbol == sym) {
                        let borrow = if a.easy_to_borrow {
                            "easy_to_borrow"
                        } else {
                            "hard_to_borrow"
                        };
                        let body = json!({
                            "symbol": a.symbol,
                            "tradable": a.tradable,
                            "shortable": a.shortable,
                            "borrow_status": borrow,
                        });
                        return (200, body.to_string());
                    }
                }
            }
        }
    }
    (404, r#"{"message":"not found"}"#.to_string())
}

/// The answer to a request missing either auth header.
pub fn unauthorized() -> (u16, String) {
    (401, r#"{"message":"forbidden."}"#.to_string())
}

/// Serve seeded bars; `page_token` is the offset of the next bar in the filtered, ordered list.
fn bars(query: &str, state: &State) -> (u16, String) {
    let param = |k: &str| {
        query
            .split('&')
            .filter_map(|kv| kv.split_once('='))
            .find(|(name, _)| *name == k)
            .map(|(_, v)| v.replace("%2C", ",").replace("%2c", ","))
    };
    let symbols: Vec<String> = param("symbols")
        .map(|s| s.split(',').map(str::to_string).collect())
        .unwrap_or_default();
    let feed = param("feed").unwrap_or_else(|| "sip".to_string());
    if state.denied_feeds.contains(&feed) {
        return (
            403,
            r#"{"message":"subscription does not permit querying recent SIP data"}"#.to_string(),
        );
    }
    let adjustment = param("adjustment").unwrap_or_else(|| "raw".to_string());
    let start = param("start").map(|s| s.chars().take(10).collect::<String>());
    let end = param("end").map(|s| s.chars().take(10).collect::<String>());
    let limit = param("limit")
        .and_then(|l| l.parse::<usize>().ok())
        .unwrap_or(1000)
        .clamp(1, 10000);
    let offset = param("page_token")
        .and_then(|t| t.parse::<usize>().ok())
        .unwrap_or(0);
    let mut matched: Vec<_> = state
        .bars
        .iter()
        .filter(|b| b.adjustment == adjustment && b.feed == feed)
        .filter(|b| symbols.contains(&b.symbol))
        .filter(|b| start.as_ref().is_none_or(|s| b.date.as_str() >= s.as_str()))
        .filter(|b| end.as_ref().is_none_or(|e| b.date.as_str() <= e.as_str()))
        .collect();
    matched.sort_by(|a, b| (&a.symbol, &a.date).cmp(&(&b.symbol, &b.date)));
    let page: Vec<_> = matched.iter().skip(offset).take(limit).collect();
    let next = offset + page.len();
    let mut out = serde_json::Map::new();
    for b in page {
        let entry = out
            .entry(b.symbol.clone())
            .or_insert_with(|| serde_json::Value::Array(Vec::new()));
        if let Some(arr) = entry.as_array_mut() {
            arr.push(json!({
                "t": format!("{}T05:00:00Z", b.date),
                "o": b.open, "h": b.high, "l": b.low, "c": b.close,
                "v": b.volume, "n": 1, "vw": b.close,
            }));
        }
    }
    let token = if next < matched.len() {
        json!(next.to_string())
    } else {
        serde_json::Value::Null
    };
    (
        200,
        json!({"bars": out, "next_page_token": token}).to_string(),
    )
}
