//! K28 (R-456): a rejection reason is written in decision JSON as its fixed name.

mod common;

use common::*;
use pt_core::signal::{RiskEstimates, SignalSet};
use pt_core::types::{Positions, Side, Weights};
use pt_engine::decision::{decision_to_json, Decision};
use pt_risk::pretrade::Rejection;
use std::collections::BTreeMap;

#[test]
fn each_rejection_is_written_as_its_name_in_order() {
    // R-456
    let rejected = vec![
        (order("A", Side::Buy, 1, 1, 100), Rejection::NoEquity),
        (order("B", Side::Buy, 2, 1, 100), Rejection::OrderNotional),
        (order("C", Side::Buy, 3, 1, 100), Rejection::WeightCap),
        (order("D", Side::Buy, 4, 1, 100), Rejection::Gross),
    ];
    let decision = Decision {
        signals: SignalSet {
            asof: d(2024, 1, 2),
            signals: BTreeMap::new(),
            ineligible: BTreeMap::new(),
        },
        risk: RiskEstimates {
            asof: d(2024, 1, 2),
            vols: BTreeMap::new(),
            cov: BTreeMap::new(),
            ineligible: BTreeMap::new(),
        },
        weights: Weights::new(),
        target: Positions::new(),
        orders: vec![],
        rejected,
    };
    let json: serde_json::Value = serde_json::from_str(&decision_to_json(&decision)).unwrap();
    let list = json["rejected"].as_array().unwrap();
    let reasons: Vec<&str> = list.iter().map(|e| e["reason"].as_str().unwrap()).collect();
    assert_eq!(reasons, ["NoEquity", "OrderNotional", "WeightCap", "Gross"]);
    let symbols: Vec<&str> = list
        .iter()
        .map(|e| e["order"]["symbol"].as_str().unwrap())
        .collect();
    assert_eq!(symbols, ["A", "B", "C", "D"]);
}
