"""EWMA volatility estimator (R-089; Time Series Momentum p13, eq. 1; formulas.md TSM-1).

The estimator is the exponentially weighted variance of daily returns with a centre of mass of
60 sessions, so the decay is ``delta = 60 / 61``. The most recent return has weight
``(1 - delta)``, the one before it ``(1 - delta) * delta``, and so on.

Lag: the caller passes the returns up to the last session visible in the view, and the estimate
uses all of them. That is the paper's ``t - 1`` rule seen from the decision instant.

Conventions chosen where the paper and R-089 differ or are silent (formulas.md FQ-9):

- Units: per session, not annualized. The paper's factor 261 is not applied; callers that need an
  annual figure multiply the variance by their own periods per year.
- Finite history: the weights are renormalized to sum to 1 over the returns given (the same as
  ``pandas.Series.ewm(com=60, adjust=True).var(bias=True)``), not truncated.
- Mean: the exponentially weighted mean, computed with the same renormalized weights, is
  subtracted. The variance is the biased weighted variance (no small-sample correction).
- Minimum history: none. One return gives a variance of 0.0; an empty input gives NaN.
  Non-finite returns are not accepted and raise ``ValueError``.
"""

from __future__ import annotations

import math
from collections.abc import Sequence

import numpy as np
import numpy.typing as npt

CENTRE_OF_MASS = 60.0
DECAY = CENTRE_OF_MASS / (CENTRE_OF_MASS + 1.0)


def _weights(n: int) -> npt.NDArray[np.float64]:
    """Normalized weights, oldest return first, newest last."""
    exponents = np.arange(n - 1, -1, -1, dtype=np.float64)
    raw = DECAY**exponents
    return raw / raw.sum()


def ewma_variance(returns: Sequence[float] | npt.NDArray[np.float64]) -> float:
    """Per-session exponentially weighted variance of ``returns`` (oldest first)."""
    values = np.asarray(returns, dtype=np.float64)
    if values.ndim != 1:
        raise ValueError("returns must be one-dimensional")
    if values.size == 0:
        return math.nan
    if not np.all(np.isfinite(values)):
        raise ValueError("returns must be finite")
    weights = _weights(values.size)
    mean = float(np.dot(weights, values))
    return float(np.dot(weights, (values - mean) ** 2))


def ewma_volatility(returns: Sequence[float] | npt.NDArray[np.float64]) -> float:
    """Per-session EWMA volatility: the square root of :func:`ewma_variance`."""
    return math.sqrt(ewma_variance(returns))
