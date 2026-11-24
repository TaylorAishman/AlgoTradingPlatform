-- 0001: market data tables (architecture 5.4).
-- Prices are TEXT holding the canonical decimal string, quantities INTEGER,
-- dates `YYYY-MM-DD`, instants RFC 3339 UTC.

-- The single calendar source: one row per trading session.
CREATE TABLE calendar_sessions (
    date TEXT NOT NULL PRIMARY KEY
) STRICT;

-- Daily bars, append-only and versioned by fetch time (D-08).
-- A new row is written only when values differ from the latest version.
CREATE TABLE bars (
    symbol     TEXT    NOT NULL,
    date       TEXT    NOT NULL,
    adjustment TEXT    NOT NULL CHECK (adjustment IN ('raw', 'all')),
    feed       TEXT    NOT NULL,
    fetched_at TEXT    NOT NULL,
    open       TEXT    NOT NULL,
    high       TEXT    NOT NULL,
    low        TEXT    NOT NULL,
    close      TEXT    NOT NULL,
    volume     INTEGER NOT NULL CHECK (volume >= 0),
    PRIMARY KEY (symbol, date, adjustment, feed, fetched_at)
) STRICT;

-- Asset flags as fetched from the broker, versioned by fetch time.
CREATE TABLE asset_snapshots (
    symbol         TEXT    NOT NULL,
    fetched_at     TEXT    NOT NULL,
    tradable       INTEGER NOT NULL CHECK (tradable IN (0, 1)),
    shortable      INTEGER NOT NULL CHECK (shortable IN (0, 1)),
    easy_to_borrow INTEGER NOT NULL CHECK (easy_to_borrow IN (0, 1)),
    PRIMARY KEY (symbol, fetched_at)
) STRICT;
