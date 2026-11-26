//! R-270: the owner checklist shows what is done and what is pending.

use chrono::NaiveDate;
use pt_gate::owner::{FirstOrderApproval, OwnerFile, Stage};
use pt_gate::status::{gate_status, Facts};

const IDS: [&str; 12] = [
    "api_keys",
    "calendar",
    "bars",
    "stage_ingest",
    "stage_dry_run",
    "alert_webhook",
    "account_flat",
    "first_order_approval",
    "stage_first_order",
    "strategy_allowlist",
    "stage_paper",
    "validation_signoff",
];

fn no_facts() -> Facts {
    Facts {
        keys_present: false,
        calendar_loaded: false,
        bars_loaded: false,
        first_order_sent: false,
    }
}

fn all_facts() -> Facts {
    Facts {
        keys_present: true,
        calendar_loaded: true,
        bars_loaded: true,
        first_order_sent: true,
    }
}

fn full_owner() -> OwnerFile {
    OwnerFile {
        stage: Stage::Paper,
        strategy_allowlist: vec!["tsmom-v1".to_string()],
        account_flat_confirmed_on: NaiveDate::from_ymd_opt(2026, 12, 1),
        first_order: Some(FirstOrderApproval {
            symbol: "SPY".to_string(),
            side: "buy".to_string(),
            qty: 1,
            max_qty: 1,
            valid_on: NaiveDate::from_ymd_opt(2026, 12, 2).unwrap(),
        }),
        validation_signoff: Some("signed".to_string()),
        alert_webhook_url: Some("https://hooks.example.test/x".to_string()),
    }
}

#[test]
fn items_come_in_the_documented_order_each_with_an_instruction() {
    // R-270
    let items = gate_status(&OwnerFile::default(), &no_facts());
    let ids: Vec<&str> = items.iter().map(|i| i.id).collect();
    assert_eq!(ids, IDS);
    for item in &items {
        assert!(
            !item.instruction.is_empty(),
            "{} has no instruction",
            item.id
        );
    }
}

#[test]
fn default_owner_file_and_no_facts_leave_every_item_pending() {
    // R-270
    let items = gate_status(&OwnerFile::default(), &no_facts());
    for item in &items {
        assert!(!item.done, "{} should be pending", item.id);
    }
}

#[test]
fn paper_stage_with_everything_set_and_all_facts_marks_every_item_done() {
    // R-270
    let items = gate_status(&full_owner(), &all_facts());
    assert_eq!(items.len(), 12);
    for item in &items {
        assert!(item.done, "{} should be done", item.id);
        assert!(!item.instruction.is_empty());
    }
}

#[test]
fn dry_run_stage_marks_the_ingest_and_dry_run_stage_items_done_only() {
    // R-270 (stage items follow the stage; see also R-382)
    let owner = OwnerFile {
        stage: Stage::DryRun,
        ..OwnerFile::default()
    };
    let items = gate_status(&owner, &no_facts());
    let done: Vec<&str> = items.iter().filter(|i| i.done).map(|i| i.id).collect();
    assert_eq!(done, ["stage_ingest", "stage_dry_run"]);
}
