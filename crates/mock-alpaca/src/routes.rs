//! Route handling: maps a recorded request to a status and JSON body.

use serde_json::json;

use crate::state::{fmt_dec, parse_dec, Order, Recorded, State};

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
    if req.method == "DELETE" {
        if let Some(id) = req.path.strip_prefix("/v2/orders/") {
            if let Some(o) = state.orders.iter_mut().find(|o| o.id == id) {
                if is_terminal(&o.status) {
                    return (422, r#"{"message":"order is not cancelable"}"#.to_string());
                }
                o.status = "canceled".to_string();
                return (204, String::new());
            }
        }
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
            "/v2/orders" => {
                let status = query_param(&req.query, "status").unwrap_or_else(|| "open".into());
                let after = query_param(&req.query, "after")
                    .and_then(|a| chrono::DateTime::parse_from_rfc3339(&a).ok());
                let limit = query_param(&req.query, "limit")
                    .and_then(|l| l.parse::<usize>().ok())
                    .unwrap_or(100)
                    .clamp(1, 500);
                let mut matched: Vec<&Order> = state
                    .orders
                    .iter()
                    .filter(|o| match status.as_str() {
                        "all" => true,
                        "closed" => is_terminal(&o.status),
                        _ => !is_terminal(&o.status),
                    })
                    .filter(|o| {
                        after.is_none_or(|a| {
                            chrono::DateTime::parse_from_rfc3339(&o.submitted_at)
                                .is_ok_and(|t| t > a)
                        })
                    })
                    .collect();
                matched.sort_by(|a, b| a.submitted_at.cmp(&b.submitted_at));
                if query_param(&req.query, "direction").as_deref() != Some("asc") {
                    matched.reverse();
                }
                let list: Vec<_> = matched.into_iter().take(limit).map(order_json).collect();
                return (200, serde_json::Value::Array(list).to_string());
            }
            "/v2/positions" => {
                let list: Vec<_> = state
                    .positions
                    .iter()
                    .map(|(sym, q)| json!({"symbol": sym, "qty": fmt_dec(*q)}))
                    .collect();
                return (200, serde_json::Value::Array(list).to_string());
            }
            "/v2/account/activities/FILL" => {
                let after = query_param(&req.query, "after")
                    .and_then(|a| chrono::DateTime::parse_from_rfc3339(&a).ok());
                let size = query_param(&req.query, "page_size")
                    .and_then(|l| l.parse::<usize>().ok())
                    .unwrap_or(100)
                    .clamp(1, 100);
                let mut matched: Vec<_> = state.activities.iter().collect();
                if query_param(&req.query, "direction").as_deref() == Some("desc") {
                    matched.reverse();
                }
                if let Some(token) = query_param(&req.query, "page_token") {
                    match matched.iter().position(|a| a.id == token) {
                        Some(i) => {
                            matched.drain(..=i);
                        }
                        None => {
                            return (422, json!({"message": "unknown page_token"}).to_string());
                        }
                    }
                }
                if let Some(t) = after {
                    matched.retain(|a| {
                        chrono::DateTime::parse_from_rfc3339(&a.transaction_time)
                            .is_ok_and(|x| x > t)
                    });
                }
                let list: Vec<_> = matched
                    .into_iter()
                    .take(size)
                    .map(|a| {
                        json!({
                            "id": a.id,
                            "order_id": a.order_id,
                            "symbol": a.symbol,
                            "side": a.side,
                            "qty": a.qty,
                            "price": a.price,
                            "transaction_time": a.transaction_time,
                            "type": a.kind,
                            "activity_type": "FILL",
                        })
                    })
                    .collect();
                return (200, serde_json::Value::Array(list).to_string());
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

/// The percent-decoded value of query parameter `key`.
fn query_param(query: &str, key: &str) -> Option<String> {
    let (_, raw) = query
        .split('&')
        .filter_map(|kv| kv.split_once('='))
        .find(|(name, _)| *name == key)?;
    let bytes = raw.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let hex = bytes
            .get(i + 1..i + 3)
            .and_then(|h| std::str::from_utf8(h).ok())
            .and_then(|h| u8::from_str_radix(h, 16).ok());
        match (bytes[i], hex) {
            (b'%', Some(b)) => {
                out.push(b);
                i += 3;
            }
            (b, _) => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8(out).ok()
}

/// Whether an order status is final (the order is no longer open).
pub(crate) fn is_terminal(status: &str) -> bool {
    matches!(
        status,
        "filled" | "canceled" | "expired" | "rejected" | "replaced"
    )
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
        "filled_avg_price": o.filled_avg_price,
    })
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
    let Some(want) = parse_dec(&qty) else {
        return refuse("invalid order qty");
    };
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
    if side == "sell" {
        let held = state
            .positions
            .iter()
            .find(|(s, _)| *s == symbol)
            .map_or(0, |(_, q)| *q);
        let shortable = state
            .assets
            .iter()
            .find(|a| a.symbol == symbol)
            .is_some_and(|a| a.shortable);
        let message = if held > 0 && want > held {
            Some("insufficient qty available for order")
        } else if held <= 0 && !shortable {
            Some("asset cannot be sold short: insufficient qty available for order")
        } else {
            None
        };
        if let Some(m) = message {
            return (403, json!({"message": m}).to_string());
        }
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
        filled_notional: 0,
        filled_avg_price: None,
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
