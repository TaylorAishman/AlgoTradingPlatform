//! R-068: every run of the binary is logged.

use chrono::{DateTime, TimeZone, Utc};
use pt_store::db::Store;

fn t(h: u32, m: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 12, 1, h, m, 0).unwrap()
}

#[test]
fn started_run_is_listed_without_a_finish_time() {
    // R-068
    let store = Store::open_in_memory().unwrap();
    let id = store.start_run("paper", "abc", "0.1.0", t(20, 0)).unwrap();
    let runs = store.runs().unwrap();
    assert_eq!(runs.len(), 1);
    let run = &runs[0];
    assert_eq!(run.id, id);
    assert_eq!(run.started_at, t(20, 0));
    assert_eq!(run.finished_at, None);
    assert_eq!(run.stage, "paper");
    assert_eq!(run.config_hash, "abc");
    assert_eq!(run.code_version, "0.1.0");
}

#[test]
fn finished_run_shows_status_phases_and_finish_time() {
    // R-068
    let store = Store::open_in_memory().unwrap();
    let id = store.start_run("paper", "abc", "0.1.0", t(20, 0)).unwrap();
    store
        .finish_run(id, "ok", "ingest,settle", t(20, 5))
        .unwrap();
    let runs = store.runs().unwrap();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].id, id);
    assert_eq!(runs[0].status, "ok");
    assert_eq!(runs[0].phases, "ingest,settle");
    assert_eq!(runs[0].finished_at, Some(t(20, 5)));
    assert_eq!(runs[0].started_at, t(20, 0));
}

#[test]
fn finishing_one_run_leaves_the_other_unfinished() {
    // R-068
    let store = Store::open_in_memory().unwrap();
    let a = store.start_run("paper", "abc", "0.1.0", t(20, 0)).unwrap();
    let b = store.start_run("ingest", "def", "0.1.0", t(21, 0)).unwrap();
    assert_ne!(a, b);
    store.finish_run(b, "ok", "ingest", t(21, 1)).unwrap();
    let runs = store.runs().unwrap();
    assert_eq!(runs.len(), 2);
    let first = runs.iter().find(|r| r.id == a).unwrap();
    let second = runs.iter().find(|r| r.id == b).unwrap();
    assert_eq!(first.finished_at, None);
    assert_eq!(second.finished_at, Some(t(21, 1)));
    assert_eq!(second.stage, "ingest");
}
