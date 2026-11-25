//! V05 (R-369): a backtest that fails stores and registers nothing.

mod common;

use common::{config, date, request, store_with};
use pt_core::clock::FixedClock;
use pt_engine::error::EngineError;
use pt_store::error::StoreError;
use pt_validate::error::ValidateError;
use pt_validate::runner::run_and_record;
use rust_decimal::Decimal;

#[test]
fn a_missing_bar_fails_the_run_and_leaves_no_backtest_or_trial() {
    // R-369
    let store = store_with(&[("S3", date(2024, 6, 12))], Decimal::ONE);
    let clock = FixedClock(common::fetched_at());

    let err = run_and_record(&store, &config(), &request(), &clock).expect_err("run must fail");
    assert!(
        matches!(err, ValidateError::Engine(EngineError::MissingBar { .. })),
        "unexpected error: {err:?}"
    );

    assert!(store.trials("tsmom-v1").expect("trials").is_empty());
    assert!(matches!(
        store.backtest_equity(1),
        Err(StoreError::NotFound(_))
    ));
}
