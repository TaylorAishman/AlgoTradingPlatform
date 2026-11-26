-- 0005: additive fixes (architecture 5.4, F-03, F-16; R-415, R-416).
-- Existing rows keep their values: off_ledger defaults to 0 and the backtest
-- capital columns start NULL for runs written before this migration.

ALTER TABLE orders ADD COLUMN off_ledger INTEGER NOT NULL DEFAULT 0 CHECK (off_ledger IN (0, 1));
ALTER TABLE backtest_runs ADD COLUMN initial_capital TEXT;
ALTER TABLE backtest_runs ADD COLUMN capital_source TEXT;

-- Asset snapshots are history: rows are only ever inserted.
CREATE TRIGGER asset_snapshots_no_update BEFORE UPDATE ON asset_snapshots
BEGIN
    SELECT RAISE(ABORT, 'asset_snapshots is append-only');
END;

CREATE TRIGGER asset_snapshots_no_delete BEFORE DELETE ON asset_snapshots
BEGIN
    SELECT RAISE(ABORT, 'asset_snapshots is append-only');
END;
