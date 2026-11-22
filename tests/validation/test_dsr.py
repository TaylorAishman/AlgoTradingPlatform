"""Tests for atp.validation.dsr (R-199 to R-201)."""

from __future__ import annotations

import math

import numpy as np
import pandas as pd
import pytest
from scipy.stats import norm

from atp.validation.dsr import (
    deflated_sharpe,
    deflated_sharpe_from_moments,
    effective_trials,
    probabilistic_sharpe,
)


def test_psr_formula() -> None:
    rng = np.random.default_rng(1)
    x = rng.standard_t(5, size=800) * 0.01 + 0.0008
    n = x.size
    m = x.mean()
    sr = m / x.std(ddof=1)
    c = x - m
    skew = (c**3).mean() / (c**2).mean() ** 1.5
    kurt = (c**4).mean() / (c**2).mean() ** 2  # plain kurtosis, 3 for a Normal
    bench = 0.02
    expected = norm.cdf(
        (sr - bench) * math.sqrt(n - 1) / math.sqrt(1 - skew * sr + (kurt - 1) / 4 * sr**2)
    )
    assert probabilistic_sharpe(pd.Series(x), bench) == pytest.approx(expected, abs=1e-10)


def test_paper_example() -> None:
    # Moments of the paper's example (p9-10), per period with 250 observations per year (FQ-1).
    sr = 2.5 / math.sqrt(250)
    var = 0.5 / 250
    args = (sr, 1250, -3.0, 10.0)
    assert deflated_sharpe_from_moments(*args, 46, var) == pytest.approx(0.9505, abs=0.0005)
    assert deflated_sharpe_from_moments(*args, 100, var) == pytest.approx(0.9004, abs=0.0005)


def test_deflated_sharpe_series_and_single_trial() -> None:
    rng = np.random.default_rng(2)
    r = pd.Series(rng.normal(0.001, 0.01, 1000))
    assert deflated_sharpe(r, 1.0, 0.01) == pytest.approx(probabilistic_sharpe(r, 0.0))
    assert deflated_sharpe(r, 50.0, 0.0004) < probabilistic_sharpe(r, 0.0)


def test_effective_trials() -> None:
    rng = np.random.default_rng(3)
    col = rng.normal(size=500)
    same = pd.DataFrame({f"c{i}": col for i in range(7)})
    assert effective_trials(same) == pytest.approx(1.0)
    indep = pd.DataFrame(rng.normal(size=(5000, 20)))
    assert abs(effective_trials(indep) - 20) < 1
    assert 1.0 <= effective_trials(indep) <= 20.0
