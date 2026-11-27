//! R-469, R-470, R-269, R-265: the kill-switch authorisation (decision G-53).

use chrono::NaiveDate;
use pt_gate::error::GateError;
use pt_gate::gate::{AuthContext, Gate, OrderSummary};
use pt_gate::owner::{OwnerFile, Stage};

const STAGES: [Stage; 5] = [
    Stage::Backtest,
    Stage::Ingest,
    Stage::DryRun,
    Stage::FirstOrder,
    Stage::Paper,
];

/// A gate with no flat confirmation and an empty allowlist.
fn bare_gate(stage: Stage) -> Gate {
    Gate::new(OwnerFile {
        stage,
        ..OwnerFile::default()
    })
}

fn order(id: &str, symbol: &str, side: &str, qty: u64) -> OrderSummary {
    OrderSummary {
        client_order_id: id.to_string(),
        symbol: symbol.to_string(),
        side: side.to_string(),
        qty,
    }
}

fn flatten_spy(side: &str, qty: u64) -> OrderSummary {
    order("pt-flatten-20261202-SPY", "SPY", side, qty)
}

#[test]
fn full_flatten_of_a_long_position_is_permitted_in_every_stage() {
    // R-469
    for stage in STAGES {
        let o = flatten_spy("sell", 10);
        let permit = bare_gate(stage)
            .authorize_kill_switch(&o, 10)
            .unwrap_or_else(|e| panic!("{stage:?}: {e:?}"));
        assert!(permit.matches(&o), "{stage:?}");
    }
}

#[test]
fn partial_flatten_is_permitted_in_every_stage() {
    // R-469
    for stage in STAGES {
        let o = flatten_spy("sell", 4);
        let permit = bare_gate(stage).authorize_kill_switch(&o, 10).unwrap();
        assert!(permit.matches(&o), "{stage:?}");
    }
}

#[test]
fn buy_to_cover_a_short_position_is_permitted_in_every_stage() {
    // R-469
    for stage in STAGES {
        let o = order("pt-flatten-20261202-TLT-r1", "TLT", "buy", 5);
        let permit = bare_gate(stage).authorize_kill_switch(&o, -5).unwrap();
        assert!(permit.matches(&o), "{stage:?}");
    }
}

#[test]
fn orders_that_could_open_or_grow_a_position_are_refused() {
    // R-470, in stage Paper as in stage Backtest
    for stage in [Stage::Paper, Stage::Backtest] {
        let g = bare_gate(stage);
        let cases: [(&str, OrderSummary, i64); 6] = [
            ("buy against +10", flatten_spy("buy", 5), 10),
            ("sell against -5", flatten_spy("sell", 5), -5),
            ("qty 11 against +10", flatten_spy("sell", 11), 10),
            ("qty 0", flatten_spy("sell", 0), 10),
            ("position 0", flatten_spy("sell", 1), 0),
            (
                "non-flatten id",
                order("pt-tsmom-v1-20261130-SPY-1", "SPY", "sell", 10),
                10,
            ),
        ];
        for (name, o, pos) in cases {
            assert_eq!(
                g.authorize_kill_switch(&o, pos).unwrap_err(),
                GateError::NotRiskReducing,
                "{stage:?} {name}"
            );
        }
    }
}

#[test]
fn authorize_refuses_flatten_orders_below_first_order_stage() {
    // R-269: authorize returns StageForbids even for a risk-reducing flatten
    let ctx = AuthContext {
        strategy_id: "tsmom-v1".to_string(),
        today: NaiveDate::from_ymd_opt(2026, 12, 2).unwrap(),
        halted: false,
        risk_reducing: true,
        is_first_order: false,
        first_order_already_sent: false,
    };
    for stage in [Stage::Backtest, Stage::Ingest, Stage::DryRun] {
        assert_eq!(
            bare_gate(stage)
                .authorize(&flatten_spy("sell", 10), &ctx)
                .unwrap_err(),
            GateError::StageForbids,
            "{stage:?}"
        );
    }
}

#[test]
fn kill_switch_permit_is_bound_to_its_order() {
    // R-265: the permit matches only the order it was issued for
    let o = flatten_spy("sell", 10);
    let permit = bare_gate(Stage::Paper)
        .authorize_kill_switch(&o, 10)
        .unwrap();
    assert!(!permit.matches(&flatten_spy("sell", 4)));
    assert!(!permit.matches(&order("pt-flatten-20261202-QQQ", "QQQ", "sell", 10)));
}
