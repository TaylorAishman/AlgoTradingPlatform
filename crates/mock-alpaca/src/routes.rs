//! Route handling: maps a recorded request to a status and JSON body.

use serde_json::json;

use crate::state::{Order, Recorded, State};

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
pub fn respond(req: &Recorded, state: &mut State) -> (u16, String) {
    if req.method == "POST" && req.path == "/v2/orders" {
        return submit(&req.body, state);
    }
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
            "/v2/orders:by_client_order_id" => {
                let wanted = req
                    .query
                    .split('&')
                    .filter_map(|kv| kv.split_once('='))
                    .find(|(name, _)| *name == "client_order_id")
                    .map(|(_, v)| v);
                if let Some(o) = state
                    .orders
                    .iter()
                    .find(|o| Some(o.client_order_id.as_str()) == wanted)
                {
                    return (200, order_json(o).to_string());
                }
            }
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

/// The order JSON shape served by the order routes.
fn order_json(o: &Order) -> serde_json::Value {
    json!({
        "id": o.id,
        "client_order_id": o.client_order_id,
        "symbol": o.symbol,
        "qty": o.qty,
        "side": o.side,
        "type": o.order_type,
        "time_in_force": o.time_in_force,
        "status": o.status,
        "submitted_at": o.submitted_at,
        "filled_qty": o.filled_qty,
    })
}

/// Whether `s` is a plain non-negative decimal such as `10` or `0.5`.
fn is_plain_decimal(s: &str) -> bool {
    let (int, frac) = s.split_once('.').unwrap_or((s, ""));
    let digits = |p: &str| p.bytes().all(|b| b.is_ascii_digit());
    !int.is_empty() && digits(int) && digits(frac)
}

/// Accept a new order, or refuse it with 422 if malformed or a duplicate client order id.
fn submit(body: &str, state: &mut State) -> (u16, String) {
    let refuse = |msg: &str| (422, json!({"message": msg}).to_string());
    let Ok(v) = serde_json::from_str::<serde_json::Value>(body) else {
        return refuse("invalid order body");
    };
    let field = |k: &str| v.get(k).and_then(|x| x.as_str()).map(str::to_string);
    let (Some(symbol), Some(qty), Some(side), Some(order_type), Some(time_in_force)) = (
        field("symbol"),
        field("qty"),
        field("side"),
        field("type"),
        field("time_in_force"),
    ) else {
        return refuse("missing order field");
    };
    if !is_plain_decimal(&qty) {
        return refuse("invalid order qty");
    }
    if side != "buy" && side != "sell" {
        return refuse("invalid order side");
    }
    let seq = state.orders.len() + 1;
    let id = format!("00000000-0000-4000-8000-{seq:012}");
    let client_order_id = field("client_order_id").unwrap_or_else(|| id.clone());
    if state
        .orders
        .iter()
        .any(|o| o.client_order_id == client_order_id)
    {
        return refuse("client_order_id must be unique");
    }
    let order = Order {
        id,
        client_order_id,
        symbol,
        qty,
        side,
        order_type,
        time_in_force,
        status: "accepted".to_string(),
        submitted_at: state.stamp(seq),
        filled_qty: "0".to_string(),
    };
    let text = order_json(&order).to_string();
    state.orders.push(order);
    (200, text)
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
