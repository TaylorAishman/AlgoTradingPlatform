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
            _ => {}
        }
    }
    (404, r#"{"message":"not found"}"#.to_string())
}

/// The answer to a request missing either auth header.
pub fn unauthorized() -> (u16, String) {
    (401, r#"{"message":"forbidden."}"#.to_string())
}
