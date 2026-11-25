//! Authenticated blocking HTTP client behind the rate limiter.

use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use crate::auth::Credentials;
use crate::endpoints::Endpoints;
use crate::error::AlpacaError;
use crate::limiter::{RateLimiter, Sleeper, SystemClock};

/// Waits before each retry after a 429 reply (R-085).
const RATE_LIMIT_WAITS_SECS: [u64; 4] = [15, 30, 45, 60];

/// Waits before each retry after a 5xx reply (R-086).
const SERVER_ERROR_WAITS_SECS: [u64; 4] = [1, 2, 4, 8];

/// Sends every request with the auth headers, after taking a rate limiter slot.
pub struct HttpClient {
    endpoints: Endpoints,
    creds: Credentials,
    limiter: RateLimiter,
    sleeper: Arc<dyn Sleeper>,
    agent: ureq::Agent,
}

impl fmt::Debug for HttpClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HttpClient")
            .field("endpoints", &self.endpoints)
            .finish_non_exhaustive()
    }
}

/// A sleeper that returns at once, so the test client never really waits.
struct NoSleep;

impl Sleeper for NoSleep {
    fn sleep(&self, _d: Duration) {}
}

/// A client for tests: both base URLs point at `url`, which must be a loopback
/// host. Any other host is refused with `EndpointNotAllowed`. It has no rate
/// limit and never sleeps (architecture 5.5).
pub fn loopback_client(url: &str) -> Result<HttpClient, AlpacaError> {
    let endpoints = Endpoints::with_overrides(Some(url), Some(url))?;
    let sleeper: Arc<dyn Sleeper> = Arc::new(NoSleep);
    let limiter = RateLimiter::new(u32::MAX, Arc::new(SystemClock::new()), sleeper.clone());
    let creds = Credentials::new("loopback-key".to_string(), "loopback-secret".to_string());
    Ok(HttpClient::new(endpoints, creds, limiter, sleeper))
}

impl HttpClient {
    pub fn new(
        endpoints: Endpoints,
        creds: Credentials,
        limiter: RateLimiter,
        sleeper: Arc<dyn Sleeper>,
    ) -> Self {
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .http_status_as_error(false)
            .build()
            .into();
        Self {
            endpoints,
            creds,
            limiter,
            sleeper,
            agent,
        }
    }

    /// The configured endpoints.
    pub fn endpoints(&self) -> &Endpoints {
        &self.endpoints
    }

    /// GET `path` on the trading base URL and return the body of a 2xx reply.
    pub fn get_trading(&self, path: &str) -> Result<String, AlpacaError> {
        let url = format!("{}{}", self.endpoints.trading(), path);
        self.get(&url)
    }

    /// GET `path_and_query` on the market data base URL and return the body of a 2xx reply.
    pub fn get_data(&self, path_and_query: &str) -> Result<String, AlpacaError> {
        let url = format!("{}{}", self.endpoints.data(), path_and_query);
        self.get(&url)
    }

    fn get(&self, url: &str) -> Result<String, AlpacaError> {
        self.with_retries(true, || self.get_once(url))
    }

    /// Sends a request through `once` until it succeeds or may not be sent
    /// again. A 429 is retried after 15, 30, 45 and 60 s (R-085). A 5xx is
    /// retried after 1, 2, 4 and 8 s only when `retry_server` is true (R-086);
    /// an order POST passes false, because the broker may already hold the
    /// order (R-101). Any other error is returned at once.
    fn with_retries(
        &self,
        retry_server: bool,
        once: impl Fn() -> Result<String, AlpacaError>,
    ) -> Result<String, AlpacaError> {
        let mut retries = 0;
        let mut server_retries = 0;
        loop {
            match once() {
                Err(AlpacaError::RateLimited) if retries < RATE_LIMIT_WAITS_SECS.len() => {
                    self.sleeper
                        .sleep(Duration::from_secs(RATE_LIMIT_WAITS_SECS[retries]));
                    retries += 1;
                }
                Err(AlpacaError::Server { .. })
                    if retry_server && server_retries < SERVER_ERROR_WAITS_SECS.len() =>
                {
                    self.sleeper
                        .sleep(Duration::from_secs(SERVER_ERROR_WAITS_SECS[server_retries]));
                    server_retries += 1;
                }
                other => return other,
            }
        }
    }

    fn get_once(&self, url: &str) -> Result<String, AlpacaError> {
        self.limiter.acquire();
        let resp = self
            .agent
            .get(url)
            .header("APCA-API-KEY-ID", self.creds.key_id())
            .header("APCA-API-SECRET-KEY", self.creds.secret_key())
            .call();
        read_reply(resp)
    }

    /// POST `json` to `path` on the trading base URL and return the body of a
    /// 2xx reply. Sent again, with the same body, only after a 429 (R-085). A
    /// 5xx, any other 4xx and a transport error are never retried (R-101).
    ///
    /// `pub(crate)` on purpose: only `submit_order`, which holds a permit, may
    /// send a POST (architecture D-17).
    pub(crate) fn post_trading(&self, path: &str, json: &str) -> Result<String, AlpacaError> {
        let url = format!("{}{}", self.endpoints.trading(), path);
        self.with_retries(false, || self.post_once(&url, json))
    }

    fn post_once(&self, url: &str, json: &str) -> Result<String, AlpacaError> {
        self.limiter.acquire();
        let resp = self
            .agent
            .post(url)
            .header("APCA-API-KEY-ID", self.creds.key_id())
            .header("APCA-API-SECRET-KEY", self.creds.secret_key())
            .header("Content-Type", "application/json")
            .send(json);
        read_reply(resp)
    }

    /// DELETE `path` on the trading base URL and return the body of a 2xx
    /// reply. A 429 and a 5xx are retried as for a GET (R-085, R-086);
    /// cancelling twice cannot create anything. Any other 4xx and a
    /// transport error are never retried.
    ///
    /// `pub(crate)` on purpose: orders are cancelled only through
    /// `cancel_order`.
    pub(crate) fn delete_trading(&self, path: &str) -> Result<String, AlpacaError> {
        let url = format!("{}{}", self.endpoints.trading(), path);
        self.with_retries(true, || self.delete_once(&url))
    }

    fn delete_once(&self, url: &str) -> Result<String, AlpacaError> {
        self.limiter.acquire();
        let resp = self
            .agent
            .delete(url)
            .header("APCA-API-KEY-ID", self.creds.key_id())
            .header("APCA-API-SECRET-KEY", self.creds.secret_key())
            .call();
        read_reply(resp)
    }
}

/// Maps a reply to the body of a 2xx, or to the error for its status.
fn read_reply(
    resp: Result<ureq::http::Response<ureq::Body>, ureq::Error>,
) -> Result<String, AlpacaError> {
    let mut resp = resp.map_err(|e| AlpacaError::Transport(e.to_string()))?;
    let status = resp.status().as_u16();
    let body = resp
        .body_mut()
        .read_to_string()
        .map_err(|e| AlpacaError::Transport(e.to_string()))?;
    match status {
        200..=299 => Ok(body),
        429 => Err(AlpacaError::RateLimited),
        500..=599 => Err(AlpacaError::Server { status }),
        _ => Err(AlpacaError::Rejected {
            status,
            message: body,
        }),
    }
}
