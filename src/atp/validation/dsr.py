"""Probabilistic and deflated Sharpe ratio (*Deflated Sharpe Ratio*, eq. 1, 2 and 9).

Conventions (formulas.md DSR-1 to DSR-4):

- All Sharpe ratios, thresholds and variances are per observation period, not annualized.
- The Sharpe ratio uses the sample standard deviation (ddof 1). Skewness is the plain moment ratio
  (biased, scipy default) and kurtosis is the plain kurtosis (3 for a Normal), not the excess
  kurtosis (FQ-2).
- ``deflated_sharpe`` applies eq. 1 as it stands for any ``n_effective`` above 1 (no floor on the
  threshold, FQ-3). At ``n_effective <= 1`` eq. 1 is undefined and the threshold is zero, so the
  deflated ratio equals the probabilistic ratio against zero (R-202).
- ``sr_variance`` is the variance of per-period Sharpe ratios across trials (FQ-4).
"""

from __future__ import annotations

import math

import numpy as np
import pandas as pd
from scipy import stats

EULER_GAMMA = 0.5772156649015329


def _moments(returns: pd.Series) -> tuple[float, int, float, float]:
    x = np.asarray(returns.dropna(), dtype=float)
    t = int(x.size)
    if t < 3:
        raise ValueError("at least 3 observations are required")
    std = float(x.std(ddof=1))
    if std == 0.0:
        raise ValueError("returns have zero variance")
    sr = float(x.mean()) / std
    skew = float(stats.skew(x))
    kurt = float(stats.kurtosis(x, fisher=False))
    return sr, t, skew, kurt


def psr_from_moments(
    sr: float, t: int, skew: float, kurt: float, sr_benchmark: float = 0.0
) -> float:
    """Eq. 2 from per-period Sharpe ratio, sample length, skewness and plain kurtosis."""
    denom_sq = 1.0 - skew * sr + (kurt - 1.0) / 4.0 * sr**2
    if denom_sq <= 0.0:
        raise ValueError("non-positive variance term in the PSR denominator")
    z = (sr - sr_benchmark) * math.sqrt(t - 1) / math.sqrt(denom_sq)
    return float(stats.norm.cdf(z))


def probabilistic_sharpe(returns: pd.Series, sr_benchmark: float) -> float:
    """Probabilistic Sharpe ratio of ``returns`` against a per-period threshold (eq. 2)."""
    sr, t, skew, kurt = _moments(returns)
    return psr_from_moments(sr, t, skew, kurt, sr_benchmark)


def expected_max_sharpe(n_effective: float, sr_variance: float) -> float:
    """Eq. 1 with a zero mean: the threshold SR_0. Zero when ``n_effective <= 1``."""
    if n_effective <= 1.0:
        return 0.0
    bracket = (1.0 - EULER_GAMMA) * stats.norm.ppf(1.0 - 1.0 / n_effective) + (
        EULER_GAMMA * stats.norm.ppf(1.0 - 1.0 / (n_effective * math.e))
    )
    return math.sqrt(sr_variance) * float(bracket)


def deflated_sharpe_from_moments(
    sr: float, t: int, skew: float, kurt: float, n_effective: float, sr_variance: float
) -> float:
    """Deflated Sharpe ratio from moments (all per-period); used for the paper's example."""
    return psr_from_moments(sr, t, skew, kurt, expected_max_sharpe(n_effective, sr_variance))


def deflated_sharpe(returns: pd.Series, n_effective: float, sr_variance: float) -> float:
    """Deflated Sharpe ratio: PSR against the expected maximum of ``n_effective`` trials."""
    sr, t, skew, kurt = _moments(returns)
    return deflated_sharpe_from_moments(sr, t, skew, kurt, n_effective, sr_variance)


def effective_trials(returns_matrix: pd.DataFrame) -> float:
    """Eq. 9: ``rho + (1 - rho) * M`` with rho the mean off-diagonal correlation, in [1, M]."""
    m = returns_matrix.shape[1]
    if m < 2:
        return 1.0
    corr = np.corrcoef(returns_matrix.to_numpy(dtype=float), rowvar=False)
    rho = (float(np.sum(corr)) - m) / (m * (m - 1))
    if math.isnan(rho):
        raise ValueError("correlation is undefined (constant column)")
    return float(min(max(rho + (1.0 - rho) * m, 1.0), float(m)))
