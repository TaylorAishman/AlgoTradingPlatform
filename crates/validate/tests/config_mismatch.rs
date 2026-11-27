//! V15 (R-528): the overfitting statistics do not depend on the configuration matching
//! the one the run was stored with.

mod common;

use common::{config, request, store};
use pt_config::hash::config_hash;
use pt_core::clock::FixedClock;
use pt_validate::overfit::{compare_to_benchmark, dsr_for_run};
use pt_validate::runner::{run_and_record, BacktestRequest};
use rust_decimal::Decimal;

fn clock() -> FixedClock {
    FixedClock(common::fetched_at())
}

#[test]
fn dsr_and_benchmark_comparison_give_the_same_values_under_a_config_with_another_hash() {
    // R-528
    let store = store();
    let cfg = config();
    let clock = clock();
    let strat = run_and_record(&store, &cfg, &request(), &clock)
        .expect("strategy run")
        .run_id;
    let bench = run_and_record(
        &store,
        &cfg,
        &BacktestRequest {
            strategy: "benchmark".into(),
            ..request()
        },
        &clock,
    )
    .expect("benchmark run")
    .run_id;

    let mut other = cfg.clone();
    other.sizing.target_vol = cfg.sizing.target_vol + Decimal::new(5, 2);
    assert_ne!(config_hash(&cfg), config_hash(&other));

    let dsr = dsr_for_run(&store, &cfg, strat, "tsmom-v1").expect("dsr original");
    let dsr_other = dsr_for_run(&store, &other, strat, "tsmom-v1").expect("dsr other config");
    assert_eq!(dsr, dsr_other);

    let cmp = compare_to_benchmark(&store, &cfg, strat, bench, 252.0).expect("cmp original");
    let cmp_other =
        compare_to_benchmark(&store, &other, strat, bench, 252.0).expect("cmp other config");
    assert_eq!(cmp, cmp_other);
}
