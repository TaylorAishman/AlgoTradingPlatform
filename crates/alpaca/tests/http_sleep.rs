mod common;

use std::sync::Arc;
use std::time::Duration;

use pt_alpaca::auth::Credentials;
use pt_alpaca::endpoints::Endpoints;
use pt_alpaca::http::HttpClient;
use pt_alpaca::limiter::{MonoClock, RateLimiter, Sleeper};

use common::Fake;

fn client(fake: &Arc<Fake>) -> HttpClient {
    let mono: Arc<dyn MonoClock> = fake.clone();
    let sleeper: Arc<dyn Sleeper> = fake.clone();
    let limiter = RateLimiter::new(200, mono, sleeper.clone()).expect("a limit of 200 is accepted");
    let endpoints =
        Endpoints::with_overrides(Some("http://127.0.0.1:1"), Some("http://127.0.0.1:1"))
            .expect("loopback");
    HttpClient::new(
        endpoints,
        Credentials::new("FAKEKEYID".to_string(), "FAKESECRET".to_string()),
        limiter,
        sleeper,
    )
}

// R-434: http.sleep(4 s) goes through the injected sleeper, once, and returns at once.
#[test]
fn http_sleep_waits_through_the_injected_sleeper() {
    let fake = Fake::new();
    let http = client(&fake);
    http.sleep(Duration::from_secs(4));
    assert_eq!(fake.sleeps(), vec![Duration::from_secs(4)]);
}
