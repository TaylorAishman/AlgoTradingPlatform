"""Multiple-testing adjustments of p-values (MT-1, *And the Cross-Section* p15-18).

``adjust_pvalues`` returns adjusted p-values; a test is rejected (a discovery) at level alpha
when its adjusted p-value is at most alpha. Each adjusted p-value is the smallest level at which
the corresponding procedure rejects that test, so this is equivalent to the stepwise rules of the
paper (Bonferroni: p <= alpha/M; Holm: step-down with alpha/(M+1-k); BHY: step-up with
k*alpha/(M*c(M))), with ``c(M) = sum_{j=1}^{M} 1/j``.

Convention: the comparison is inclusive (``<=``), as in the paper, and the p-values must be
passed as given, not recomputed from t-ratios. The index of the input series is preserved and
the original order is kept; NaN p-values are not supported.
"""

from __future__ import annotations

from typing import Literal

import numpy as np
import pandas as pd


def adjust_pvalues(p: pd.Series, method: Literal["bonferroni", "holm", "bhy"]) -> pd.Series:
    """Return the Bonferroni, Holm or BHY adjusted p-values of ``p``, in the order of ``p``."""
    values = p.to_numpy(dtype=float)
    m = values.size
    if m == 0:
        return p.astype(float).copy()
    if np.isnan(values).any():
        raise ValueError("p-values must not contain NaN")
    if ((values < 0) | (values > 1)).any():
        raise ValueError("p-values must lie in [0, 1]")

    order = np.argsort(values, kind="stable")
    sorted_p = values[order]
    ranks = np.arange(1, m + 1, dtype=float)

    if method == "bonferroni":
        adjusted_sorted = np.minimum(m * sorted_p, 1.0)
    elif method == "holm":
        adjusted_sorted = np.minimum(np.maximum.accumulate((m - ranks + 1) * sorted_p), 1.0)
    elif method == "bhy":
        c_m = float(np.sum(1.0 / ranks))
        raw = m * c_m / ranks * sorted_p
        adjusted_sorted = np.minimum(np.minimum.accumulate(raw[::-1])[::-1], 1.0)
    else:
        raise ValueError(f"unknown method: {method!r}")

    adjusted = np.empty(m, dtype=float)
    adjusted[order] = adjusted_sorted
    return pd.Series(adjusted, index=p.index, name=p.name)
