//! R-499, R-500: an edited approval gets its own attempts while no first
//! order has been accepted; once one has, a different approval is ignored
//! with a warning.

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

/// Swallows every wait so the back-off costs no time.
#[derive(Default)]
struct Recording(Mutex<Vec<Duration>>);

impl Sleeper for Recording {
    fn sleep(&self, d: Duration) {
        self.0.lock().unwrap().push(d);
    }
}

fn gate(qty: u64, max_qty: u64) -> Gate {
    Gate::new(OwnerFile {
        stage: Stage::FirstOrder,
        account_flat_confirmed_on: Some(d(2024, 2, 29)),
        first_order: Some(FirstOrderApproval {
            symbol: "SPY".to_string(),
            side: "buy".to_string(),
            qty,
            max_qty,
            valid_on: d(2024, 3, 1),
        }),
        ..OwnerFile::default()
    })
}

fn setup() -> LiveFixture {
    let mut fx = LiveFixture::new(Stage::FirstOrder);
    fx.gate = gate(1, 1);
    let sleeper: Arc<dyn Sleeper> = Arc::new(Recording::default());
    let url = fx.mock.url();
    let limiter =
        RateLimiter::new(u32::MAX, Arc::new(SystemClock::new()), sleeper.clone()).expect("limiter");
    fx.http = HttpClient::new(
        Endpoints::with_overrides(Some(&url), Some(&url)).expect("loopback"),
        Credentials::new("FAKEKEYID".to_string(), "FAKESECRET".to_string()),
        limiter,
        sleeper,
    );
    fx
}

/// Quantities of the POSTs the mock recorded, in order.
fn post_qtys(fx: &LiveFixture) -> Vec<i64> {
    fx.mock
        .requests()
        .into_iter()
        .filter(|r| r.method == "POST" && r.path.starts_with("/v2/orders"))
        .map(|r| {
            let v: serde_json::Value = serde_json::from_str(&r.body).expect("json body");
            match &v["qty"] {
                serde_json::Value::String(s) => s.parse().expect("numeric qty"),
                other => other.as_i64().expect("numeric qty"),
            }
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

fn ignored_alerts(fx: &LiveFixture) -> Vec<String> {
    fx.store
        .alerts()
        .expect("alerts")
        .into_iter()
        .filter(|a| a.1 == "first_order_ignored")
        .map(|a| a.0)
        .collect()
}

// R-499
#[test]
fn an_edited_approval_gets_its_own_attempts_after_the_old_one_died() {
    let mut fx = setup();
    fx.mock.reject_next("POST", "/v2/orders", 6, 422);
    let first = run_cycle(&fx.ctx()).expect("first run");
    assert_eq!(first.status, RunStatus::Failed);
    assert_eq!(post_qtys(&fx).len(), 6);

    fx.gate = gate(2, 5);
    let second = run_cycle(&fx.ctx()).expect("second run");

    assert_eq!(second.status, RunStatus::Ok);
    let qtys = post_qtys(&fx);
    assert_eq!(qtys.len(), 7, "one more POST");
    assert_eq!(qtys[6], 2);
    assert_eq!(
        ids_in(&fx, OrderState::Submitted),
        ["pt-first-20240301-SPY-r6"]
    );
}

// R-500
#[test]
fn a_different_approval_after_acceptance_is_ignored_with_a_warning() {
    let mut fx = setup();
    let first = run_cycle(&fx.ctx()).expect("first run");
    assert_eq!(first.status, RunStatus::Ok);
    assert_eq!(post_qtys(&fx), [1]);
    assert!(ignored_alerts(&fx).is_empty());

    fx.gate = gate(2, 5);
    let second = run_cycle(&fx.ctx()).expect("second run");

    assert_eq!(second.status, RunStatus::Ok);
    assert_eq!(post_qtys(&fx), [1], "no further POST");
    assert_eq!(ignored_alerts(&fx), ["warn"]);
}

// R-500
#[test]
fn an_unchanged_approval_after_acceptance_raises_no_ignored_alert() {
    let fx = setup();
    let first = run_cycle(&fx.ctx()).expect("first run");
    assert_eq!(first.status, RunStatus::Ok);

    let second = run_cycle(&fx.ctx()).expect("second run");

    assert_eq!(second.status, RunStatus::Ok);
    assert_eq!(post_qtys(&fx), [1]);
    assert!(ignored_alerts(&fx).is_empty());
}
