"""Definitions of the ten tables of ``$ATP_HOME/state.sqlite`` (Arch 5.12, plan note PL-6).

The eight tables of Arch 5.12 plus ``gate_reports`` and ``promotions`` (requirements I15)
live here so that every component can write state without importing another component.
A component may add its own auxiliary tables, triggers and indices from its own module.

Conventions for every column:

- timestamps are ISO-8601 text in UTC; session dates are ``YYYY-MM-DD`` text;
- quantities and prices are ``Decimal`` values stored as text, so nothing is rounded;
- series, mappings and reports are JSON text;
- booleans are integers 0 or 1.
"""

from __future__ import annotations

import sqlite3
from collections.abc import Mapping
from types import MappingProxyType

from atp.core.home import HomeLayout, ensure_layout

SCHEMA_VERSION = 1

_TABLES: dict[str, str] = {
    "trials": """
CREATE TABLE IF NOT EXISTS trials (
    trial_id        TEXT PRIMARY KEY,
    family          TEXT NOT NULL,
    role            TEXT NOT NULL CHECK (role IN ('candidate', 'baseline', 'diagnostic')),
    config_hash     TEXT NOT NULL,
    code_rev        TEXT NOT NULL,
    data_snapshot   TEXT NOT NULL,
    synthetic       INTEGER NOT NULL CHECK (synthetic IN (0, 1)),
    seed            INTEGER,
    opened_at       TEXT NOT NULL,
    status          TEXT NOT NULL,
    closed_at       TEXT,
    data_quality    TEXT CHECK (data_quality IS NULL OR data_quality IN ('sip', 'iex')),
    returns         TEXT,
    metrics         TEXT
)""",
    "runs": """
CREATE TABLE IF NOT EXISTS runs (
    run_id          TEXT PRIMARY KEY,
    kind            TEXT NOT NULL CHECK (kind IN ('backtest', 'paper')),
    trial_id        TEXT,
    config_hash     TEXT NOT NULL,
    started_at      TEXT NOT NULL,
    ended_at        TEXT,
    status          TEXT NOT NULL,
    data_quality    TEXT CHECK (data_quality IS NULL OR data_quality IN ('sip', 'iex')),
    path            TEXT NOT NULL
)""",
    "orders": """
CREATE TABLE IF NOT EXISTS orders (
    client_order_id TEXT PRIMARY KEY,
    broker_order_id TEXT,
    run_id          TEXT,
    session         TEXT NOT NULL,
    asset           TEXT NOT NULL,
    side            TEXT NOT NULL CHECK (side IN ('buy', 'sell')),
    qty             TEXT NOT NULL,
    order_type      TEXT NOT NULL,
    time_in_force   TEXT NOT NULL,
    limit_price     TEXT,
    status          TEXT NOT NULL,
    filled_qty      TEXT NOT NULL DEFAULT '0',
    reject_reason   TEXT,
    submitted_at    TEXT,
    updated_at      TEXT NOT NULL
)""",
    "fills": """
CREATE TABLE IF NOT EXISTS fills (
    fill_id         TEXT PRIMARY KEY,
    client_order_id TEXT NOT NULL,
    asset           TEXT NOT NULL,
    side            TEXT NOT NULL CHECK (side IN ('buy', 'sell')),
    qty             TEXT NOT NULL,
    price           TEXT NOT NULL,
    filled_at       TEXT NOT NULL
)""",
    "position_snapshots": """
CREATE TABLE IF NOT EXISTS position_snapshots (
    snapshot_id     INTEGER PRIMARY KEY AUTOINCREMENT,
    run_id          TEXT,
    session         TEXT NOT NULL,
    taken_at        TEXT NOT NULL,
    source          TEXT NOT NULL CHECK (source IN ('broker', 'recorded')),
    asset           TEXT NOT NULL,
    qty             TEXT NOT NULL,
    avg_entry_price TEXT,
    market_value    TEXT
)""",
    "kill_switch": """
CREATE TABLE IF NOT EXISTS kill_switch (
    id              INTEGER PRIMARY KEY CHECK (id = 1),
    engaged         INTEGER NOT NULL CHECK (engaged IN (0, 1)),
    reason          TEXT,
    set_by          TEXT,
    set_at          TEXT,
    cleared_at      TEXT
)""",
    "model_cards": """
CREATE TABLE IF NOT EXISTS model_cards (
    model_id         TEXT PRIMARY KEY,
    vendor           TEXT NOT NULL,
    knowledge_cutoff TEXT NOT NULL,
    cutoff_source    TEXT NOT NULL CHECK (length(trim(cutoff_source)) > 0)
)""",
    "job_log": """
CREATE TABLE IF NOT EXISTS job_log (
    job_id          TEXT PRIMARY KEY,
    job_name        TEXT NOT NULL,
    session         TEXT,
    started_at      TEXT NOT NULL,
    ended_at        TEXT,
    status          TEXT NOT NULL,
    exit_code       INTEGER,
    log_path        TEXT
)""",
    "gate_reports": """
CREATE TABLE IF NOT EXISTS gate_reports (
    report_id       TEXT PRIMARY KEY,
    trial_id        TEXT NOT NULL,
    family          TEXT NOT NULL,
    config_hash     TEXT NOT NULL,
    created_at      TEXT NOT NULL,
    passed          INTEGER NOT NULL CHECK (passed IN (0, 1)),
    data_quality    TEXT CHECK (data_quality IS NULL OR data_quality IN ('sip', 'iex')),
    report          TEXT NOT NULL
)""",
    "promotions": """
CREATE TABLE IF NOT EXISTS promotions (
    promotion_id    INTEGER PRIMARY KEY AUTOINCREMENT,
    strategy        TEXT NOT NULL,
    config_hash     TEXT NOT NULL,
    from_state      TEXT CHECK (
        from_state IS NULL OR from_state IN ('research', 'candidate', 'paper', 'validated')
    ),
    to_state        TEXT NOT NULL CHECK (
        to_state IN ('research', 'candidate', 'paper', 'validated')
    ),
    report_id       TEXT,
    changed_at      TEXT NOT NULL
)""",
}

TABLES: Mapping[str, str] = MappingProxyType(_TABLES)
"""Table name -> ``CREATE TABLE IF NOT EXISTS`` statement, in the order of Arch 5.12."""

TABLE_NAMES: tuple[str, ...] = tuple(_TABLES)

PROMOTION_STATES: tuple[str, ...] = ("research", "candidate", "paper", "validated")
TRIAL_ROLES: tuple[str, ...] = ("candidate", "baseline", "diagnostic")


def create_schema(conn: sqlite3.Connection) -> None:
    """Create any missing table and the single ``kill_switch`` row (off). Idempotent."""
    with conn:
        for ddl in TABLES.values():
            conn.execute(ddl)
        conn.execute("INSERT OR IGNORE INTO kill_switch (id, engaged) VALUES (1, 0)")
        conn.execute(f"PRAGMA user_version = {SCHEMA_VERSION}")


def open_state(layout: HomeLayout | None = None) -> sqlite3.Connection:
    """Open ``state.sqlite`` of ``layout`` (default: the resolved ``ATP_HOME``).

    Creates the ``ATP_HOME`` layout and the ten tables if they are missing.
    The caller closes the returned connection.
    """
    home = ensure_layout(None if layout is None else layout.root)
    conn = sqlite3.connect(home.state_db)
    try:
        create_schema(conn)
    except BaseException:
        conn.close()
        raise
    return conn
