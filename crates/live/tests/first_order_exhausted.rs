//! R-497, R-498: after the fifth retry is rejected the run fails with one
//! critical alert and no halt, and the dead approval is not tried again.

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

fn setup() -> LiveFixture {
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

fn posts(fx: &LiveFixture) -> usize {
    fx.mock
        .requests()
        .iter()
        .filter(|r| r.method == "POST" && r.path.starts_with("/v2/orders"))
        .count()
}

fn rejected_ids(fx: &LiveFixture) -> Vec<String> {
    let mut ids: Vec<String> = fx
        .store
        .orders_in_states(&[OrderState::Rejected])
        .expect("orders")
        .into_iter()
        .map(|o| o.client_order_id)
        .collect();
    ids.sort();
    ids
}

fn order_count(fx: &LiveFixture) -> usize {
    use OrderState::*;
    fx.store
        .orders_in_states(&[
            Intent,
            DryRun,
            Abandoned,
            Submitted,
            PartiallyFilled,
            Filled,
            Canceled,
            Expired,
            Rejected,
        ])
        .expect("orders")
        .len()
}

// R-497
#[test]
fn six_rejections_fail_the_run_with_one_critical_alert_and_no_halt() {
    let fx = setup();
    fx.mock.reject_next("POST", "/v2/orders", 6, 422);

    let outcome = run_cycle(&fx.ctx()).expect("run");

    assert_eq!(outcome.status, RunStatus::Failed);
    assert_eq!(
        rejected_ids(&fx),
        [
            "pt-first-20240301-SPY",
            "pt-first-20240301-SPY-r1",
            "pt-first-20240301-SPY-r2",
            "pt-first-20240301-SPY-r3",
            "pt-first-20240301-SPY-r4",
            "pt-first-20240301-SPY-r5",
        ]
    );
    assert_eq!(posts(&fx), 6);
    let alerts = fx.store.alerts().expect("alerts");
    let exhausted: Vec<_> = alerts
        .iter()
        .filter(|a| a.1 == "first_order_exhausted")
        .collect();
    assert_eq!(exhausted.len(), 1);
    assert_eq!(exhausted[0].0, "critical");
    assert!(alerts.iter().all(|a| a.1 != "run_failed"));
    assert!(fx.store.active_halts().expect("halts").is_empty());
    let runs = fx.store.runs().expect("runs");
    assert_eq!(runs.last().expect("run row").status, "failed");
}

// R-498
#[test]
fn an_approval_that_used_up_its_attempts_is_not_tried_again() {
    let fx = setup();
    fx.mock.reject_next("POST", "/v2/orders", 6, 422);
    let first = run_cycle(&fx.ctx()).expect("first run");
    assert_eq!(first.status, RunStatus::Failed);
    let posts_before = posts(&fx);
    let orders_before = order_count(&fx);
    let alerts_before = fx.store.alerts().expect("alerts").len();

    let second = run_cycle(&fx.ctx()).expect("second run");

    assert_eq!(second.status, RunStatus::Ok);
    assert_eq!(posts(&fx), posts_before, "no further POST");
    assert_eq!(order_count(&fx), orders_before);
    assert_eq!(fx.store.alerts().expect("alerts").len(), alerts_before);
}
