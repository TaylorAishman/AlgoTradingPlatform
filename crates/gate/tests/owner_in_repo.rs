//! R-262: an owner file inside the repository must be refused.

use pt_gate::error::GateError;
use pt_gate::owner::{load_owner_file, Stage};

const VALID: &str = "stage = \"ingest\"\n";

#[test]
fn existing_valid_file_under_repo_root_is_refused() {
    // R-262
    let repo = tempfile::tempdir().unwrap();
    let path = repo.path().join("owner.toml");
    std::fs::write(&path, VALID).unwrap();

    let result = load_owner_file(&path, repo.path());

    assert_eq!(result, Err(GateError::OwnerFileInRepo));
}

#[test]
fn missing_file_under_repo_root_is_refused() {
    // R-262
    let repo = tempfile::tempdir().unwrap();
    let path = repo.path().join("owner.toml");

    let result = load_owner_file(&path, repo.path());

    assert_eq!(result, Err(GateError::OwnerFileInRepo));
}

#[test]
fn file_in_nested_directory_under_repo_root_is_refused() {
    // R-262
    let repo = tempfile::tempdir().unwrap();
    let dir = repo.path().join("config").join("private");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("owner.toml");
    std::fs::write(&path, VALID).unwrap();

    let result = load_owner_file(&path, repo.path());

    assert_eq!(result, Err(GateError::OwnerFileInRepo));
}

#[test]
fn file_outside_repo_root_still_loads() {
    // R-262 (boundary: only paths under the repository are refused)
    let repo = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let path = outside.path().join("owner.toml");
    std::fs::write(&path, VALID).unwrap();

    let owner = load_owner_file(&path, repo.path()).expect("outside the repository loads");

    assert_eq!(owner.stage, Stage::Ingest);
}
