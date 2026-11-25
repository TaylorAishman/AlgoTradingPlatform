//! Tests for the age of the last good run (R-361).

use chrono::{DateTime, Duration, TimeZone, Utc};
use pt_ops::report::last_ok_run_age_hours;
use pt_store::db::Store;

fn t() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 3, 2, 22, 0, 0).unwrap()
}

// R-361: an ok run at t followed by a failed run gives the age of the ok run.
#[test]
fn age_is_measured_from_the_last_ok_run_not_a_later_failed_one() {
    let store = Store::open_in_memory().unwrap();
    let a = store
        .start_run("paper", "h", "v", t() - Duration::minutes(5))
        .unwrap();
    store.finish_run(a, "ok", "", t()).unwrap();
    let b = store
        .start_run("paper", "h", "v", t() + Duration::hours(1))
        .unwrap();
    store
        .finish_run(b, "failed", "", t() + Duration::hours(2))
        .unwrap();
    let age = last_ok_run_age_hours(&store, t() + Duration::hours(5)).unwrap();
    assert_eq!(age, Some(5.0));
}

// R-361: with no ok run the age is None.
#[test]
fn age_is_none_without_an_ok_run() {
    let store = Store::open_in_memory().unwrap();
    assert_eq!(last_ok_run_age_hours(&store, t()).unwrap(), None);
    let a = store.start_run("paper", "h", "v", t()).unwrap();
    store
        .finish_run(a, "failed", "", t() + Duration::minutes(1))
        .unwrap();
    assert_eq!(
        last_ok_run_age_hours(&store, t() + Duration::hours(3)).unwrap(),
        None
    );
}
