//! R-494, R-495: a rejected first order is tried again under new client
//! order ids, waiting 1, 2, 4, 8 and 16 seconds between tries.

mod common;

use std::sync::{Arc, Mutex};
use std::time::Duration;

use common::{d, LiveFixture};
use pt_alpaca::auth::Credentials;
use pt_alpaca::endpoints::Endpoints;
use pt_alpaca::http::HttpClient;
use pt_alpaca::limiter::{RateLimiter, Sleeper, SystemClock};
use pt_core::order_state::OrderState;
use pt_gate::gate::Gate;
use pt_gate::owner::{FirstOrderApproval, OwnerFile, Stage};
use pt_live::cycle::{run_cycle, RunStatus};

/// Records every wait it is asked for and returns at once.
#[derive(Default)]
struct Recording(Mutex<Vec<Duration>>);

impl Sleeper for Recording {
    fn sleep(&self, d: Duration) {
        self.0.lock().unwrap().push(d);
    }
}

fn setup() -> (LiveFixture, Arc<Recording>) {
    let mut fx = LiveFixture::new(Stage::FirstOrder);
    fx.gate = Gate::new(OwnerFile {
        stage: Stage::FirstOrder,
        account_flat_confirmed_on: Some(d(2024, 2, 29)),
        first_order: Some(FirstOrderApproval {
            symbol: "SPY".to_string(),
            side: "buy".to_string(),
            qty: 1,
            max_qty: 1,
            valid_on: d(2024, 3, 1),
        }),
        ..OwnerFile::default()
    });
    let rec = Arc::new(Recording::default());
    let sleeper: Arc<dyn Sleeper> = rec.clone();
    let url = fx.mock.url();
    let limiter =
        RateLimiter::new(u32::MAX, Arc::new(SystemClock::new()), sleeper.clone()).expect("limiter");
    fx.http = HttpClient::new(
        Endpoints::with_overrides(Some(&url), Some(&url)).expect("loopback"),
        Credentials::new("FAKEKEYID".to_string(), "FAKESECRET".to_string()),
        limiter,
        sleeper,
    );
    (fx, rec)
}

fn post_ids(fx: &LiveFixture) -> Vec<String> {
    fx.mock
        .requests()
        .into_iter()
        .filter(|r| r.method == "POST")
        .map(|r| {
            let v: serde_json::Value = serde_json::from_str(&r.body).expect("json body");
            v["client_order_id"].as_str().expect("id").to_string()
        })
        .collect()
}

fn ids_in(fx: &LiveFixture, state: OrderState) -> Vec<String> {
    let mut ids: Vec<String> = fx
        .store
        .orders_in_states(&[state])
        .expect("orders")
        .into_iter()
        .map(|o| o.client_order_id)
        .collect();
    ids.sort();
    ids
}

// R-494
#[test]
fn a_rejected_first_order_is_retried_under_new_ids() {
    let (fx, _) = setup();
    fx.mock.reject_next("POST", "/v2/orders", 2, 422);

    let outcome = run_cycle(&fx.ctx()).expect("run");

    assert_eq!(outcome.status, RunStatus::Ok);
    assert_eq!(
        ids_in(&fx, OrderState::Rejected),
        ["pt-first-20240301-SPY", "pt-first-20240301-SPY-r1"]
    );
    assert_eq!(
        ids_in(&fx, OrderState::Submitted),
        ["pt-first-20240301-SPY-r2"]
    );
    assert_eq!(
        post_ids(&fx),
        [
            "pt-first-20240301-SPY",
            "pt-first-20240301-SPY-r1",
            "pt-first-20240301-SPY-r2"
        ]
    );
    let rejected_alerts = fx
        .store
        .alerts()
        .expect("alerts")
        .iter()
        .filter(|a| a.1 == "order_rejected")
        .count();
    assert_eq!(rejected_alerts, 2);
}

// R-495
#[test]
fn retries_wait_one_two_four_eight_and_sixteen_seconds() {
    let (fx, rec) = setup();
    fx.mock.reject_next("POST", "/v2/orders", 5, 422);

    let outcome = run_cycle(&fx.ctx()).expect("run");

    assert_eq!(outcome.status, RunStatus::Ok);
    assert_eq!(
        ids_in(&fx, OrderState::Submitted),
        ["pt-first-20240301-SPY-r5"]
    );
    let waits: Vec<u64> = rec
        .0
        .lock()
        .unwrap()
        .iter()
        .filter(|d| **d >= Duration::from_secs(1))
        .map(|d| d.as_secs())
        .collect();
    assert_eq!(waits, [1, 2, 4, 8, 16]);
}
