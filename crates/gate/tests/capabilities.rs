//! R-263: each stage allows exactly the capabilities of the stage table.

use pt_gate::error::GateError;
use pt_gate::gate::{Capability, Gate};
use pt_gate::owner::{OwnerFile, Stage};

fn gate_at(stage: Stage) -> Gate {
    Gate::new(OwnerFile {
        stage,
        ..OwnerFile::default()
    })
}

fn expect(allowed: bool) -> Result<(), GateError> {
    if allowed {
        Ok(())
    } else {
        Err(GateError::StageForbids)
    }
}

#[test]
fn stage_table_for_the_four_non_submit_capabilities() {
    // R-263: (stage, MarketDataRead, TradingRead, RecordDryRun, CancelOrder)
    let table = [
        (Stage::Backtest, false, false, false, false),
        (Stage::Ingest, true, true, false, false),
        (Stage::DryRun, true, true, true, false),
        (Stage::FirstOrder, true, true, true, true),
        (Stage::Paper, true, true, true, true),
    ];
    for (stage, market, trading, dry_run, cancel) in table {
        let gate = gate_at(stage);
        assert_eq!(
            gate.allows(Capability::MarketDataRead),
            expect(market),
            "{stage:?} MarketDataRead"
        );
        assert_eq!(
            gate.allows(Capability::TradingRead),
            expect(trading),
            "{stage:?} TradingRead"
        );
        assert_eq!(
            gate.allows(Capability::RecordDryRun),
            expect(dry_run),
            "{stage:?} RecordDryRun"
        );
        assert_eq!(
            gate.allows(Capability::CancelOrder),
            expect(cancel),
            "{stage:?} CancelOrder"
        );
    }
}

#[test]
fn submit_order_is_allowed_only_in_first_order_and_paper() {
    // R-263
    for (stage, allowed) in [
        (Stage::Backtest, false),
        (Stage::Ingest, false),
        (Stage::DryRun, false),
        (Stage::FirstOrder, true),
        (Stage::Paper, true),
    ] {
        assert_eq!(
            gate_at(stage).allows(Capability::SubmitOrder),
            expect(allowed),
            "{stage:?} SubmitOrder"
        );
    }
}

#[test]
fn gate_reports_its_stage() {
    // R-263
    for stage in [
        Stage::Backtest,
        Stage::Ingest,
        Stage::DryRun,
        Stage::FirstOrder,
        Stage::Paper,
    ] {
        assert_eq!(gate_at(stage).stage(), stage);
    }
}

#[test]
fn stages_compare_in_the_documented_order() {
    // R-263
    assert!(Stage::Backtest < Stage::Ingest);
    assert!(Stage::Ingest < Stage::DryRun);
    assert!(Stage::DryRun < Stage::FirstOrder);
    assert!(Stage::FirstOrder < Stage::Paper);
}
