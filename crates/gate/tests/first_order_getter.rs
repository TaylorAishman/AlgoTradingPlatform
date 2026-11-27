//! R-471: the approved first order is readable from the gate.

use chrono::NaiveDate;
use pt_gate::gate::Gate;
use pt_gate::owner::{FirstOrderApproval, OwnerFile, Stage};

// R-471
#[test]
fn first_order_returns_the_approval_from_the_owner_file() {
    let approval = FirstOrderApproval {
        symbol: "SPY".to_string(),
        side: "buy".to_string(),
        qty: 3,
        max_qty: 5,
        valid_on: NaiveDate::from_ymd_opt(2026, 12, 2).unwrap(),
    };
    let gate = Gate::new(OwnerFile {
        stage: Stage::FirstOrder,
        first_order: Some(approval.clone()),
        ..OwnerFile::default()
    });
    assert_eq!(gate.first_order(), Some(&approval));
}

// R-471
#[test]
fn first_order_is_none_when_the_table_is_absent() {
    let gate = Gate::new(OwnerFile::default());
    assert_eq!(gate.first_order(), None);
}
