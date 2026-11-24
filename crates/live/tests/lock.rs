use pt_live::error::LiveError;
use pt_live::lock::acquire_run_lock;

// R-310: a second lock on the same path fails while the first is alive.
#[test]
fn second_lock_on_same_path_is_already_running() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("trader.db");
    let _first = acquire_run_lock(&db).expect("first lock");
    let second = acquire_run_lock(&db);
    assert!(matches!(second, Err(LiveError::AlreadyRunning)));
}

// R-310: after the first lock is dropped a third call succeeds.
#[test]
fn lock_can_be_taken_again_after_drop() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("trader.db");
    let first = acquire_run_lock(&db).expect("first lock");
    assert!(matches!(
        acquire_run_lock(&db),
        Err(LiveError::AlreadyRunning)
    ));
    drop(first);
    assert!(acquire_run_lock(&db).is_ok());
}

// R-310: locks on different databases do not interfere.
#[test]
fn locks_on_different_paths_are_independent() {
    let dir = tempfile::tempdir().unwrap();
    let _a = acquire_run_lock(&dir.path().join("a.db")).expect("a");
    let _b = acquire_run_lock(&dir.path().join("b.db")).expect("b");
}
