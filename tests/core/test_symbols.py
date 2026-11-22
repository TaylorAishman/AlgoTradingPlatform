from atp.testing.symbols import COMPANY_NAMES, RESERVED_PREFIX, SYMBOLS, is_reserved


def test_symbols_match_company_names() -> None:
    assert set(SYMBOLS) == set(COMPANY_NAMES)
    assert len(SYMBOLS) == len(set(SYMBOLS))


def test_symbols_are_reserved_form() -> None:
    for s in SYMBOLS:
        assert s.startswith(RESERVED_PREFIX)
        assert s.isalpha() and s.isupper()


def test_company_names_unique_and_nonempty() -> None:
    names = list(COMPANY_NAMES.values())
    assert all(names)
    assert len(names) == len(set(names))


def test_is_reserved() -> None:
    assert is_reserved(SYMBOLS[0])
    assert not is_reserved("AAPL")
