//! R-394 (exit 2 and 3): `trader run` exits 2 when it completes but is halted,
//! and 3 when another process holds the run lock, in which case it writes
//! nothing and sends nothing.

mod common;

use common::live::{env, posts, run, Env};
use common::{stderr, stdout};
use std::path::PathBuf;

fn db_path(e: &Env) -> PathBuf {
    e.dir.path().join("test.sqlite")
}

#[test]
fn run_exits_two_and_sends_no_order_while_a_halt_is_active() {
    // R-394
    let e = env();
    {
        let store = pt_store::db::Store::open(&db_path(&e)).unwrap();
        store
            .set_halt("manual", "holiday", chrono::Utc::now())
            .unwrap();
    }
    let out = run(&e, &["run"]);
    assert_eq!(
        out.status.code(),
        Some(2),
        "stdout: {} stderr: {}",
        stdout(&out),
        stderr(&out)
    );
    assert!(posts(&e).is_empty(), "POSTs: {:?}", posts(&e));
}

#[test]
fn run_exits_three_and_writes_nothing_while_another_process_holds_the_lock() {
    // R-394
    let e = env();
    let held = pt_live::lock::acquire_run_lock(&db_path(&e)).unwrap();
    let out = run(&e, &["run"]);
    assert_eq!(
        out.status.code(),
        Some(3),
        "stdout: {} stderr: {}",
        stdout(&out),
        stderr(&out)
    );
    assert!(
        e.mock.requests().is_empty(),
        "requests: {:?}",
        e.mock.requests()
    );
    assert!(
        !e.dir.path().join("reports").exists(),
        "the report directory was written"
    );
    // R-401: every stderr line is a JSON line with level, message and ts.
    let err = stderr(&out);
    let lines: Vec<&str> = err.lines().filter(|l| !l.trim().is_empty()).collect();
    assert!(!lines.is_empty(), "no stderr line for the refused run");
    for line in lines {
        let v: serde_json::Value = serde_json::from_str(line)
            .unwrap_or_else(|_| panic!("stderr line is not JSON: {line}"));
        for key in ["level", "message", "ts"] {
            assert!(v.get(key).is_some(), "missing {key} in {line}");
        }
    }
    drop(held);
}

#[test]
fn run_succeeds_again_once_the_lock_is_released() {
    // R-394: the lock is freed on release, so the next run is not refused.
    let e = env();
    let held = pt_live::lock::acquire_run_lock(&db_path(&e)).unwrap();
    assert_eq!(run(&e, &["run"]).status.code(), Some(3));
    drop(held);
    let out = run(&e, &["run"]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "stdout: {} stderr: {}",
        stdout(&out),
        stderr(&out)
    );
    assert_eq!(posts(&e).len(), 2);
}

#[test]
fn a_refused_run_creates_no_database_file() {
    // R-534: the lock is taken before the store is opened.
    let e = env();
    let db = db_path(&e);
    assert!(!db.exists(), "the fixture already has a database");
    let held = pt_live::lock::acquire_run_lock(&db).unwrap();
    let out = run(&e, &["run"]);
    assert_eq!(
        out.status.code(),
        Some(3),
        "stdout: {} stderr: {}",
        stdout(&out),
        stderr(&out)
    );
    for suffix in ["", "-wal", "-shm"] {
        let mut name = db.as_os_str().to_owned();
        name.push(suffix);
        let p = PathBuf::from(name);
        assert!(!p.exists(), "{} was created", p.display());
    }
    drop(held);
}
