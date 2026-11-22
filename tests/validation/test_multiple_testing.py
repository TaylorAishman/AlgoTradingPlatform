"""R-208: Table 4 of *And the Cross-Section* (formulas.md MT-1).

Convention: a test is a discovery when its adjusted p-value is <= 0.05.
"""

import numpy as np
import pandas as pd
import pytest

from atp.validation.multiple_testing import adjust_pvalues

# Table 4 p-values in percent, used as printed (not recomputed from t-ratios).
P_PERCENT = [4.66, 0.85, 2.71, 0.05, 3.00, 0.84, 0.00, 0.00, 0.60, 1.28]


def _discoveries(method: str) -> list[int]:
    p = pd.Series(P_PERCENT, index=range(1, 11)) / 100.0
    adjusted = adjust_pvalues(p, method)  # type: ignore[arg-type]
    return [int(i) for i in adjusted.index[adjusted <= 0.05]]


def test_table_4() -> None:
    assert _discoveries("bonferroni") == [4, 7, 8]
    assert _discoveries("holm") == [4, 7, 8, 9]
    assert _discoveries("bhy") == [2, 4, 6, 7, 8, 9]


def test_adjusted_values_and_order() -> None:
    p = pd.Series([0.04, 0.01, 0.03], index=["a", "b", "c"])
    bonf = adjust_pvalues(p, "bonferroni")
    assert list(bonf.index) == ["a", "b", "c"]
    np.testing.assert_allclose(bonf.to_numpy(), [0.12, 0.03, 0.09])
    holm = adjust_pvalues(p, "holm")
    np.testing.assert_allclose(holm.to_numpy(), [0.06, 0.03, 0.06])
    bhy = adjust_pvalues(p, "bhy")
    assert (bhy >= p).all() and (bhy <= 1).all()


def test_invalid_input() -> None:
    with pytest.raises(ValueError):
        adjust_pvalues(pd.Series([0.1]), "nope")  # type: ignore[arg-type]
    with pytest.raises(ValueError):
        adjust_pvalues(pd.Series([1.5]), "holm")
