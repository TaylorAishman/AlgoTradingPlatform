//! Authenticated blocking HTTP client behind the rate limiter.

use std::fmt;
use std::sync::Arc;

use crate::auth::Credentials;
use crate::endpoints::Endpoints;
use crate::error::AlpacaError;
use crate::limiter::{RateLimiter, Sleeper};

/// Sends every request with the auth headers, after taking a rate limiter slot.
pub struct HttpClient {
    endpoints: Endpoints,
    creds: Credentials,
    limiter: RateLimiter,
    // Used by the retry steps to wait between attempts.
    #[allow(dead_code)]
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

    fn get(&self, url: &str) -> Result<String, AlpacaError> {
        self.limiter.acquire();
        let mut resp = self
            .agent
            .get(url)
            .header("APCA-API-KEY-ID", self.creds.key_id())
            .header("APCA-API-SECRET-KEY", self.creds.secret_key())
            .call()
            .map_err(|e| AlpacaError::Transport(e.to_string()))?;
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
}
