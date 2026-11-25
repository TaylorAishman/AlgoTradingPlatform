//! R-071: a repeated trial must not raise the trial count.

use pt_store::db::Store;
use pt_store::ops::TrialRow;

fn trial(family: &str, hash: &str, sharpe: f64) -> TrialRow {
    TrialRow {
        family: family.to_string(),
        trial_hash: hash.to_string(),
        sharpe,
        n_obs: 500,
        runs: 1,
    }
}

#[test]
fn repeated_trial_counts_once_and_keeps_latest_sharpe() {
    // R-071
    let store = Store::open_in_memory().unwrap();
    store
        .register_trial(&trial("tsmom-v1", "h1", 0.25))
        .unwrap();
    store.register_trial(&trial("tsmom-v1", "h1", 0.5)).unwrap();
    store
        .register_trial(&trial("tsmom-v1", "h2", 0.75))
        .unwrap();

    let rows = store.trials("tsmom-v1").unwrap();
    assert_eq!(rows.len(), 2);
    let h1 = rows.iter().find(|r| r.trial_hash == "h1").unwrap();
    assert_eq!(h1.runs, 2);
    assert_eq!(h1.sharpe, 0.5);
    assert_eq!(h1.n_obs, 500);
    let h2 = rows.iter().find(|r| r.trial_hash == "h2").unwrap();
    assert_eq!(h2.runs, 1);
    assert_eq!(h2.sharpe, 0.75);
}

#[test]
fn other_family_has_no_trials() {
    // R-071
    let store = Store::open_in_memory().unwrap();
    store
        .register_trial(&trial("tsmom-v1", "h1", 0.25))
        .unwrap();
    assert!(store.trials("bh-volscaled-v1").unwrap().is_empty());
}

#[test]
fn same_hash_in_different_families_is_separate() {
    // R-071
    let store = Store::open_in_memory().unwrap();
    store.register_trial(&trial("a", "h1", 0.1)).unwrap();
    store.register_trial(&trial("b", "h1", 0.2)).unwrap();
    assert_eq!(store.trials("a").unwrap().len(), 1);
    assert_eq!(store.trials("a").unwrap()[0].runs, 1);
    assert_eq!(store.trials("b").unwrap()[0].sharpe, 0.2);
}
