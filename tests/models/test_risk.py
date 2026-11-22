"""Tests for the risk models (D01: EWMA volatility)."""

from __future__ import annotations

import math

import numpy as np
import pandas as pd
import pytest

from atp.models.volatility import DECAY, ewma_variance, ewma_volatility


def _closed_form(returns: list[float]) -> float:
    """Direct evaluation of TSM-1 with renormalized weights and the weighted mean."""
    n = len(returns)
    w = [(1 - DECAY) * DECAY**i for i in range(n)]  # i = 0 is the newest return
    total = sum(w)
    r = returns[::-1]
    mean = sum(wi * ri for wi, ri in zip(w, r, strict=True)) / total
    return sum(wi * (ri - mean) ** 2 for wi, ri in zip(w, r, strict=True)) / total


def test_ewma_volatility_closed_form() -> None:
    # Conventions (formulas.md FQ-9): per session, weights renormalized, weighted mean
    # subtracted, biased variance, no minimum history.
    rng = np.random.default_rng(7)
    returns = rng.normal(0.0, 0.01, 400)

    assert DECAY == 60 / 61
    # centre of mass of the weights is 60: delta / (1 - delta)
    assert DECAY / (1 - DECAY) == pytest.approx(60.0, abs=1e-12)

    expected = _closed_form(list(returns))
    assert ewma_variance(returns) == pytest.approx(expected, abs=1e-12, rel=0)
    assert ewma_volatility(returns) == pytest.approx(math.sqrt(expected), abs=1e-12, rel=0)

    # Independent reference: pandas ewm with com=60, adjust=True, biased variance.
    pandas_var = pd.Series(returns).ewm(com=60, adjust=True).var(bias=True).iloc[-1]
    assert ewma_variance(returns) == pytest.approx(pandas_var, abs=1e-12, rel=0)

    # Only returns up to the last visible session count: the estimate on a shorter view
    # equals the closed form on that view.
    assert ewma_variance(returns[:-1]) == pytest.approx(
        _closed_form(list(returns[:-1])), abs=1e-12, rel=0
    )


def test_ewma_volatility_edge_cases() -> None:
    assert math.isnan(ewma_volatility([]))
    assert ewma_volatility([0.01]) == 0.0
    assert ewma_volatility([0.02, 0.02, 0.02]) == pytest.approx(0.0, abs=1e-12)
    with pytest.raises(ValueError):
        ewma_volatility([0.01, float("nan")])
