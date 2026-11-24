-- 0002: trading tables (architecture 5.4).
-- Same encodings as 0001: money and prices TEXT holding the canonical decimal
-- string, quantities INTEGER, dates `YYYY-MM-DD`, instants RFC 3339 UTC.

-- One decision per strategy, signal date and mode; inputs and outputs as JSON.
CREATE TABLE decisions (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    strategy_id  TEXT NOT NULL,
    signal_date  TEXT NOT NULL,
    trade_date   TEXT,
    mode         TEXT NOT NULL CHECK (mode IN ('dry_run', 'paper')),
    status       TEXT NOT NULL,
    inputs_json  TEXT NOT NULL,
    outputs_json TEXT NOT NULL,
    created_at   TEXT NOT NULL,
    UNIQUE (strategy_id, signal_date, mode)
) STRICT;

-- Orders keyed by the client order id we generate.
CREATE TABLE orders (
    client_order_id TEXT    NOT NULL PRIMARY KEY,
    decision_id     INTEGER REFERENCES decisions (id),
    symbol          TEXT    NOT NULL,
    side            TEXT    NOT NULL,
    qty             INTEGER NOT NULL CHECK (qty >= 0),
    leg             INTEGER NOT NULL CHECK (leg >= 0),
    state           TEXT    NOT NULL,
    broker_order_id TEXT,
    filled_qty      INTEGER NOT NULL CHECK (filled_qty >= 0),
    reference_price TEXT    NOT NULL,
    trade_date      TEXT    NOT NULL
) STRICT;

-- Every order state change, in insertion order.
CREATE TABLE order_events (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    client_order_id TEXT    NOT NULL REFERENCES orders (client_order_id),
    from_state      TEXT,
    to_state        TEXT    NOT NULL,
    filled_qty      INTEGER NOT NULL CHECK (filled_qty >= 0),
    broker_order_id TEXT,
    at              TEXT    NOT NULL
) STRICT;

-- Broker fills keyed by activity id. Append-only.
CREATE TABLE fills (
    activity_id     TEXT    NOT NULL PRIMARY KEY,
    broker_order_id TEXT    NOT NULL,
    symbol          TEXT    NOT NULL,
    side            TEXT    NOT NULL,
    qty             INTEGER NOT NULL CHECK (qty >= 0),
    price           TEXT    NOT NULL,
    at              TEXT    NOT NULL
) STRICT;

-- Cash and position movements per book. Append-only.
CREATE TABLE ledger_entries (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    book       TEXT    NOT NULL CHECK (book IN ('paper', 'shadow')),
    date       TEXT    NOT NULL,
    kind       TEXT    NOT NULL,
    symbol     TEXT,
    qty        INTEGER NOT NULL,
    price      TEXT,
    cash_delta TEXT    NOT NULL,
    ref_id     TEXT
) STRICT;

-- One equity snapshot per book and date; `recorded_at` keeps its first value
-- on replace (enforced by the writer).
CREATE TABLE equity_snapshots (
    book            TEXT NOT NULL CHECK (book IN ('paper', 'shadow')),
    date            TEXT NOT NULL,
    cash            TEXT NOT NULL,
    positions_value TEXT NOT NULL,
    equity          TEXT NOT NULL,
    recorded_at     TEXT NOT NULL,
    PRIMARY KEY (book, date)
) STRICT;
