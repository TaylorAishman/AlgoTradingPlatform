//! The owner checklist: what the owner has done and what is still pending.

use crate::owner::{OwnerFile, Stage};

/// Facts the owner file cannot tell: what the binary found on this machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Facts {
    pub keys_present: bool,
    pub calendar_loaded: bool,
    pub bars_loaded: bool,
    pub first_order_sent: bool,
}

/// One line of the checklist.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChecklistItem {
    pub id: &'static str,
    pub done: bool,
    pub instruction: &'static str,
}

fn item(id: &'static str, done: bool, instruction: &'static str) -> ChecklistItem {
    ChecklistItem {
        id,
        done,
        instruction,
    }
}

/// The twelve checklist items in the documented order.
pub fn gate_status(owner: &OwnerFile, facts: &Facts) -> Vec<ChecklistItem> {
    let at = |s: Stage| owner.stage >= s;
    vec![
        item(
            "api_keys",
            facts.keys_present,
            "Export APCA_API_KEY_ID and APCA_API_SECRET_KEY, or set PT_SECRETS_FILE.",
        ),
        item(
            "calendar",
            facts.calendar_loaded,
            "Sync the trading calendar into the store.",
        ),
        item(
            "bars",
            facts.bars_loaded,
            "Ingest daily bars for the universe.",
        ),
        item(
            "stage_ingest",
            at(Stage::Ingest),
            "Set stage = \"ingest\" in the owner file.",
        ),
        item(
            "stage_dry_run",
            at(Stage::DryRun),
            "Set stage = \"dry_run\" in the owner file.",
        ),
        item(
            "alert_webhook",
            owner.alert_webhook_url.is_some(),
            "Set alert_webhook_url in the owner file or PT_ALERT_WEBHOOK_URL.",
        ),
        item(
            "account_flat",
            owner.account_flat_confirmed_on.is_some(),
            "Reset the paper account and set account_flat_confirmed_on in the owner file.",
        ),
        item(
            "first_order_approval",
            owner.first_order.is_some(),
            "Add a [first_order] table with symbol, side, max_qty and valid_on.",
        ),
        item(
            "stage_first_order",
            at(Stage::FirstOrder),
            "Set stage = \"first_order\" in the owner file.",
        ),
        item(
            "strategy_allowlist",
            !owner.strategy_allowlist.is_empty(),
            "Add the strategy id to strategy_allowlist in the owner file.",
        ),
        item(
            "stage_paper",
            at(Stage::Paper),
            "Set stage = \"paper\" in the owner file.",
        ),
        item(
            "validation_signoff",
            owner.validation_signoff.is_some(),
            "Read the sign-off report and record the outcome in validation_signoff.",
        ),
    ]
}
