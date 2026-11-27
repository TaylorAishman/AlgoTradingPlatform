//! The report inputs, assembled in one place for `run` and `report`.

use chrono::Utc;
use pt_config::config::Config;
use pt_ops::error::OpsError;
use pt_ops::report::{write_status, ReportInputs};
use pt_store::db::Store;
use std::path::PathBuf;

/// Write `status.md` and `status.json` and return their paths.
///
/// The fidelity section is empty for now.
pub fn write_report(
    store: &Store,
    cfg: &Config,
    stage: &str,
    validation_signoff: Option<&str>,
) -> Result<(PathBuf, PathBuf), OpsError> {
    let inputs = ReportInputs {
        store,
        cfg,
        stage,
        fidelity: &[],
        validation_signoff,
        now: Utc::now(),
    };
    write_status(&inputs)
}
