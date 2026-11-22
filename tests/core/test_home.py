"""Tests of the ``ATP_HOME`` layout and the tables of ``state.sqlite`` (task A05)."""

from __future__ import annotations

import sqlite3
from pathlib import Path

import pytest

from atp.core.home import DEFAULT_HOME, ENV_VAR, ensure_layout, home_layout, resolve_home
from atp.core.state import TABLE_NAMES, open_state

EXPECTED_TABLES = {
    "trials",
    "runs",
    "orders",
    "fills",
    "position_snapshots",
    "kill_switch",
    "model_cards",
    "job_log",
    "gate_reports",
    "promotions",
}
EXPECTED_DIRS = {"data", "models", "runs", "llm_cache", "logs", "bench"}


def _user_tables(conn: sqlite3.Connection) -> set[str]:
    rows = conn.execute(
        "SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%'"
    ).fetchall()
    return {name for (name,) in rows}


def test_default_home_is_var_in_working_directory(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    monkeypatch.delenv(ENV_VAR, raising=False)
    monkeypatch.chdir(tmp_path)
    assert DEFAULT_HOME == Path("var")
    assert resolve_home() == (tmp_path / "var").resolve()
    assert resolve_home({ENV_VAR: "  "}) == (tmp_path / "var").resolve()


def test_env_var_selects_home(atp_home: Path) -> None:
    assert resolve_home() == atp_home.resolve()
    assert home_layout().root == atp_home.resolve()


def test_layout_paths_do_not_touch_disk(atp_home: Path) -> None:
    layout = home_layout()
    assert layout.state_db == atp_home.resolve() / "state.sqlite"
    assert {d.name for d in layout.directories()} == EXPECTED_DIRS
    assert all(d.parent == layout.root for d in layout.directories())
    assert list(atp_home.iterdir()) == []


def test_ensure_layout_creates_directories_idempotently(atp_home: Path) -> None:
    ensure_layout()
    ensure_layout()
    assert {p.name for p in atp_home.iterdir()} == EXPECTED_DIRS
    assert all(p.is_dir() for p in atp_home.iterdir())


def test_ensure_layout_creates_missing_root(tmp_path: Path) -> None:
    root = tmp_path / "a" / "b"
    layout = ensure_layout(root)
    assert layout.root == root.resolve()
    assert {p.name for p in root.iterdir()} == EXPECTED_DIRS


def test_run_dir(atp_home: Path) -> None:
    layout = home_layout()
    assert layout.run_dir("r-001") == atp_home.resolve() / "runs" / "r-001"
    for bad in ["", ".", "..", "a/b", "../x", "a\\b", "a\x00b"]:
        with pytest.raises(ValueError):
            layout.run_dir(bad)


def test_state_sqlite_holds_the_ten_tables(atp_home: Path) -> None:
    assert set(TABLE_NAMES) == EXPECTED_TABLES
    assert len(TABLE_NAMES) == 10
    conn = open_state()
    try:
        assert _user_tables(conn) == EXPECTED_TABLES
    finally:
        conn.close()
    names = {p.name for p in atp_home.iterdir()}
    assert names == EXPECTED_DIRS | {"state.sqlite"}


def test_open_state_is_idempotent_and_keeps_rows(atp_home: Path) -> None:
    conn = open_state()
    with conn:
        conn.execute(
            "INSERT INTO job_log (job_id, job_name, started_at, status) VALUES (?, ?, ?, ?)",
            ("j1", "decide", "2026-01-02T21:00:00Z", "running"),
        )
    conn.close()
    conn = open_state(home_layout(atp_home))
    try:
        assert _user_tables(conn) == EXPECTED_TABLES
        assert conn.execute("SELECT job_id FROM job_log").fetchall() == [("j1",)]
        assert conn.execute("SELECT COUNT(*) FROM kill_switch").fetchone() == (1,)
    finally:
        conn.close()


def test_kill_switch_is_one_row_starting_off(atp_home: Path) -> None:
    conn = open_state()
    try:
        assert conn.execute("SELECT id, engaged FROM kill_switch").fetchall() == [(1, 0)]
        with pytest.raises(sqlite3.IntegrityError):
            conn.execute("INSERT INTO kill_switch (id, engaged) VALUES (2, 1)")
    finally:
        conn.close()


def test_model_card_needs_cutoff_source(atp_home: Path) -> None:
    conn = open_state()
    try:
        conn.execute(
            "INSERT INTO model_cards VALUES (?, ?, ?, ?)",
            ("m1", "vendor", "2024-06-01", "vendor model page"),
        )
        for empty in ["", "   "]:
            with pytest.raises(sqlite3.IntegrityError):
                conn.execute(
                    "INSERT INTO model_cards VALUES (?, ?, ?, ?)",
                    ("m2", "vendor", "2024-06-01", empty),
                )
    finally:
        conn.close()


def test_enumerated_columns_are_checked(atp_home: Path) -> None:
    conn = open_state()
    try:
        with pytest.raises(sqlite3.IntegrityError):
            conn.execute(
                "INSERT INTO trials (trial_id, family, role, config_hash, code_rev,"
                " data_snapshot, synthetic, opened_at, status)"
                " VALUES ('t1', 'f', 'placebo', 'h', 'r', 's', 0, 'now', 'open')"
            )
        with pytest.raises(sqlite3.IntegrityError):
            conn.execute(
                "INSERT INTO promotions (strategy, config_hash, to_state, changed_at)"
                " VALUES ('s', 'h', 'live', 'now')"
            )
    finally:
        conn.close()
