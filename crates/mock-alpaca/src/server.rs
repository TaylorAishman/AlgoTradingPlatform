//! The loopback HTTP server.

use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use crate::routes;
pub use crate::state::Recorded;
use crate::state::{Asset, Bar, Clock, State};

/// A mock Alpaca server bound to 127.0.0.1 on a free port.
pub struct MockAlpaca {
    server: Arc<tiny_http::Server>,
    state: Arc<Mutex<State>>,
    port: u16,
    worker: Option<JoinHandle<()>>,
}

impl MockAlpaca {
    /// Bind 127.0.0.1 on a free port and start serving.
    pub fn start() -> MockAlpaca {
        let server = Arc::new(tiny_http::Server::http("127.0.0.1:0").expect("bind loopback port"));
        let port = server
            .server_addr()
            .to_ip()
            .expect("server listens on an IP address")
            .port();
        let state = Arc::new(Mutex::new(State::default()));
        let worker = {
            let server = Arc::clone(&server);
            let state = Arc::clone(&state);
            std::thread::spawn(move || {
                for mut request in server.incoming_requests() {
                    let full = request.url().to_string();
                    let (path, query) = match full.split_once('?') {
                        Some((p, q)) => (p.to_string(), q.to_string()),
                        None => (full, String::new()),
                    };
                    let mut body = String::new();
                    let _ = request.as_reader().read_to_string(&mut body);
                    let authed = ["APCA-API-KEY-ID", "APCA-API-SECRET-KEY"]
                        .iter()
                        .all(|name| {
                            request
                                .headers()
                                .iter()
                                .any(|h| h.field.equiv(name) && !h.value.as_str().is_empty())
                        });
                    let rec = Recorded {
                        method: request.method().to_string(),
                        path,
                        query,
                        body,
                    };
                    let mut st = state.lock().unwrap_or_else(|e| e.into_inner());
                    let injected = match st.fail_next {
                        Some((n, code)) if n > 0 => {
                            st.fail_next = (n > 1).then_some((n - 1, code));
                            Some(code)
                        }
                        _ => None,
                    };
                    let (status, text) = if let Some(code) = injected {
                        (code, r#"{"message":"injected failure"}"#.to_string())
                    } else if authed {
                        routes::respond(&rec, &mut st)
                    } else {
                        routes::unauthorized()
                    };
                    st.requests.push(rec);
                    drop(st);
                    let mut response =
                        tiny_http::Response::from_string(text).with_status_code(status);
                    if let Ok(h) = tiny_http::Header::from_bytes("Content-Type", "application/json")
                    {
                        response.add_header(h);
                    }
                    let _ = request.respond(response);
                }
            })
        };
        MockAlpaca {
            server,
            state,
            port,
            worker: Some(worker),
        }
    }

    /// Base URL serving trading and data routes.
    pub fn url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    /// Seed the account served at `/v2/account`; `cash` is also the equity.
    pub fn seed_account(&self, cash: &str) {
        self.state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .account_cash = Some(cash.to_string());
    }

    /// Set the clock served at `/v2/clock`.
    pub fn set_clock(&self, timestamp: &str, is_open: bool, next_open: &str, next_close: &str) {
        self.state.lock().unwrap_or_else(|e| e.into_inner()).clock = Some(Clock {
            timestamp: timestamp.to_string(),
            is_open,
            next_open: next_open.to_string(),
            next_close: next_close.to_string(),
        });
    }

    /// Seed the session dates (YYYY-MM-DD) served at `/v2/calendar`.
    pub fn seed_sessions(&self, dates: &[&str]) {
        self.state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .sessions = dates.iter().map(|d| d.to_string()).collect();
    }

    /// Seed an asset served at `/v2/assets/<symbol>`; `easy_to_borrow` picks the borrow status.
    pub fn seed_asset(&self, symbol: &str, tradable: bool, shortable: bool, easy_to_borrow: bool) {
        let mut st = self.state.lock().unwrap_or_else(|e| e.into_inner());
        st.assets.retain(|a| a.symbol != symbol);
        st.assets.push(Asset {
            symbol: symbol.to_string(),
            tradable,
            shortable,
            easy_to_borrow,
        });
    }

    /// Seed daily bars for one adjustment and feed, appended to earlier seeds.
    /// Each tuple is (symbol, date, open, high, low, close, volume).
    #[allow(clippy::type_complexity)]
    pub fn seed_bars(
        &self,
        adjustment: &str,
        feed: &str,
        bars: Vec<(String, String, f64, f64, f64, f64, u64)>,
    ) {
        let mut st = self.state.lock().unwrap_or_else(|e| e.into_inner());
        for (symbol, date, open, high, low, close, volume) in bars {
            st.bars.push(Bar {
                adjustment: adjustment.to_string(),
                feed: feed.to_string(),
                symbol,
                date,
                open,
                high,
                low,
                close,
                volume,
            });
        }
    }

    /// Answer bars requests for `feed` with 403.
    pub fn deny_feed(&self, feed: &str) {
        let mut st = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if !st.denied_feeds.iter().any(|f| f == feed) {
            st.denied_feeds.push(feed.to_string());
        }
    }

    /// Answer the next `n` requests with `status`; they are still recorded.
    pub fn fail_next(&self, n: u32, status: u16) {
        self.state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .fail_next = (n > 0).then_some((n, status));
    }

    /// Every request received so far, oldest first.
    pub fn requests(&self) -> Vec<Recorded> {
        self.state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .requests
            .clone()
    }
}

impl Drop for MockAlpaca {
    fn drop(&mut self) {
        self.server.unblock();
        if let Some(w) = self.worker.take() {
            let _ = w.join();
        }
    }
}
