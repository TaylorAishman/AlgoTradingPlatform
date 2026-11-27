//! R-485: `flatten` works in every stage, without the flat confirmation and
//! without the allowlist.

mod common;

use common::LiveFixture;
use pt_gate::gate::Gate;
use pt_gate::owner::{OwnerFile, Stage};
use pt_live::killswitch::flatten;

// R-485
#[test]
fn flatten_closes_both_positions_in_every_stage_without_confirmation_or_allowlist() {
    for stage in [
        Stage::Backtest,
        Stage::Ingest,
        Stage::DryRun,
        Stage::FirstOrder,
    ] {
        let mut fx = LiveFixture::new(stage);
        fx.gate = Gate::new(OwnerFile {
            stage,
            strategy_allowlist: Vec::new(),
            account_flat_confirmed_on: None,
            ..OwnerFile::default()
        });
        fx.mock.seed_position("SPY", 10);
        fx.mock.seed_position("TLT", -5);

        let report = flatten(&fx.ctx())
            .unwrap_or_else(|e| panic!("flatten in stage {stage:?} failed: {e:?}"));

        assert_eq!(report.accepted, 2, "stage {stage:?}");
        let posts = fx
            .mock
            .requests()
            .iter()
            .filter(|r| r.method == "POST" && r.path == "/v2/orders")
            .count();
        assert_eq!(posts, 2, "closing POSTs in stage {stage:?}");
        assert!(
            fx.store
                .active_halts()
                .expect("halts")
                .iter()
                .any(|h| h.reason == "flattened"),
            "flattened halt in stage {stage:?}"
        );
    }
}
