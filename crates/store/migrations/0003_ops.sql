-- 0003: shadow and operations tables (architecture 5.4).
-- Same encodings as 0001: money and prices TEXT holding the canonical decimal
-- string, quantities INTEGER, dates `YYYY-MM-DD`, instants RFC 3339 UTC.
-- Returns, basis points and Sharpe ratios are statistics, stored as REAL.

-- Paper return, shadow return and their difference per session.
CREATE TABLE shadow_daily (
    date          TEXT NOT NULL PRIMARY KEY,
    paper_return  REAL NOT NULL,
    shadow_return REAL NOT NULL,
    diff          REAL NOT NULL
) STRICT;

-- Fill price against the reference price, per order.
CREATE TABLE slippage (
    client_order_id TEXT    NOT NULL PRIMARY KEY,
    reference_price TEXT    NOT NULL,
    avg_fill_price  TEXT    NOT NULL,
    qty             INTEGER NOT NULL CHECK (qty >= 0),
    bps             REAL    NOT NULL
) STRICT;

-- One row per run of the binary; `phases` is JSON.
CREATE TABLE runs (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    started_at   TEXT NOT NULL,
    finished_at  TEXT,
    stage        TEXT NOT NULL,
    status       TEXT NOT NULL,
    config_hash  TEXT NOT NULL,
    code_version TEXT NOT NULL,
    phases       TEXT NOT NULL
) STRICT;

-- Trading halts; a halt is active while `cleared_at` is NULL.
CREATE TABLE halts (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    reason     TEXT NOT NULL,
    detail     TEXT NOT NULL,
    set_at     TEXT NOT NULL,
    cleared_at TEXT,
    clear_note TEXT
) STRICT;

-- Alerts raised, and whether delivery succeeded (0 or 1).
CREATE TABLE alerts (
    id        INTEGER PRIMARY KEY AUTOINCREMENT,
    level     TEXT    NOT NULL,
    code      TEXT    NOT NULL,
    message   TEXT    NOT NULL,
    delivered INTEGER NOT NULL CHECK (delivered IN (0, 1)),
    at        TEXT    NOT NULL
) STRICT;

-- Trial registry for multiple-testing corrections.
CREATE TABLE trials (
    family     TEXT    NOT NULL,
    trial_hash TEXT    NOT NULL,
    sharpe     REAL    NOT NULL,
    n_obs      INTEGER NOT NULL CHECK (n_obs >= 0),
    runs       INTEGER NOT NULL CHECK (runs >= 0),
    PRIMARY KEY (family, trial_hash)
) STRICT;

-- Stored research backtests.
CREATE TABLE backtest_runs (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    family      TEXT NOT NULL,
    trial_hash  TEXT NOT NULL,
    config_hash TEXT NOT NULL,
    created_at  TEXT NOT NULL
) STRICT;

-- Equity curve of a stored backtest.
CREATE TABLE backtest_equity (
    run_id INTEGER NOT NULL REFERENCES backtest_runs (id),
    date   TEXT    NOT NULL,
    equity TEXT    NOT NULL,
    PRIMARY KEY (run_id, date)
) STRICT;
