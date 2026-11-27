//! R-496: a first order the broker answered with an error but holds is
//! adopted, not duplicated.

mod common;

use common::{d, LiveFixture};
use pt_core::order_state::OrderState;
use pt_gate::gate::Gate;
use pt_gate::owner::{FirstOrderApproval, OwnerFile, Stage};
use pt_live::cycle::{run_cycle, RunStatus};

const FIRST_ID: &str = "pt-first-20240301-SPY";

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
    fx
}

fn posts(fx: &LiveFixture) -> usize {
    fx.mock
        .requests()
        .iter()
        .filter(|r| r.method == "POST" && r.path.starts_with("/v2/orders"))
        .count()
}

// R-496
#[test]
fn a_first_order_the_broker_rejected_but_holds_is_adopted_not_duplicated() {
    let fx = setup();
    fx.mock.reject_but_keep_next_order(422);

    let outcome = run_cycle(&fx.ctx()).expect("run");

    assert_eq!(outcome.status, RunStatus::Ok);
    assert_eq!(posts(&fx), 1, "exactly one POST");
    let row = fx
        .store
        .order(FIRST_ID)
        .expect("read")
        .expect("first order row");
    assert_eq!(row.state, OrderState::Submitted);
    let broker_id = row.broker_order_id.expect("broker order id stored");
    assert!(!broker_id.is_empty());
    assert!(
        fx.store
            .order(&format!("{FIRST_ID}-r1"))
            .expect("read")
            .is_none(),
        "no retry row"
    );
}
