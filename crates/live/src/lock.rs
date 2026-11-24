//! A per-database run lock so only one run works on a database at a time.

use std::ffi::OsString;
use std::fs::{File, OpenOptions, TryLockError};
use std::path::Path;

use crate::error::LiveError;

/// Holds the run lock; the lock is released when this value is dropped.
#[derive(Debug)]
pub struct RunLock {
    _file: File,
}

/// Take an exclusive lock on `<db_path>.lock`. The OS frees it if the process dies.
pub fn acquire_run_lock(db_path: &Path) -> Result<RunLock, LiveError> {
    let mut name: OsString = db_path.as_os_str().to_owned();
    name.push(".lock");
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(Path::new(&name))
        .map_err(|e| LiveError::Invalid(format!("cannot open run lock: {e}")))?;
    match file.try_lock() {
        Ok(()) => Ok(RunLock { _file: file }),
        Err(TryLockError::WouldBlock) => Err(LiveError::AlreadyRunning),
        Err(TryLockError::Error(e)) => {
            Err(LiveError::Invalid(format!("cannot take run lock: {e}")))
        }
    }
}
