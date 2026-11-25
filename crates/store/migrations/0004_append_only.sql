-- 0004: append-only triggers (architecture 5.4, D-08; R-057).
-- Bars, fills and ledger entries are history: rows are only ever inserted.
-- Any UPDATE or DELETE against these tables aborts the statement.

CREATE TRIGGER bars_no_update BEFORE UPDATE ON bars
BEGIN
    SELECT RAISE(ABORT, 'bars is append-only');
END;

CREATE TRIGGER bars_no_delete BEFORE DELETE ON bars
BEGIN
    SELECT RAISE(ABORT, 'bars is append-only');
END;

CREATE TRIGGER fills_no_update BEFORE UPDATE ON fills
BEGIN
    SELECT RAISE(ABORT, 'fills is append-only');
END;

CREATE TRIGGER fills_no_delete BEFORE DELETE ON fills
BEGIN
    SELECT RAISE(ABORT, 'fills is append-only');
END;

CREATE TRIGGER ledger_entries_no_update BEFORE UPDATE ON ledger_entries
BEGIN
    SELECT RAISE(ABORT, 'ledger_entries is append-only');
END;

CREATE TRIGGER ledger_entries_no_delete BEFORE DELETE ON ledger_entries
BEGIN
    SELECT RAISE(ABORT, 'ledger_entries is append-only');
END;
