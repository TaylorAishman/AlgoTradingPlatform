use pt_gate::owner::Stage;

// R-468: a stage has one name, defined with the stage.
#[test]
fn stage_as_str_names_the_five_stages() {
    assert_eq!(Stage::Backtest.as_str(), "backtest");
    assert_eq!(Stage::Ingest.as_str(), "ingest");
    assert_eq!(Stage::DryRun.as_str(), "dry_run");
    assert_eq!(Stage::FirstOrder.as_str(), "first_order");
    assert_eq!(Stage::Paper.as_str(), "paper");
}
