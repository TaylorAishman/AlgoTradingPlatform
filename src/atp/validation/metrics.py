"""Performance metrics and the excess-return Sharpe ratio (R-197, R-198).

Definitions follow *Multi-Period Trading via Convex Optimization* p22-23. Returns are simple
per-period returns. Every function is pure and returns plain floats.
"""

from __future__ import annotations

from collections.abc import Mapping
from dataclasses import asdict, dataclass

import numpy as np
import pandas as pd

DEFAULT_PERIODS_PER_YEAR = 252


def mean_return(returns: pd.Series) -> float:
    """Arithmetic mean of the per-period returns."""
    return float(np.mean(returns.to_numpy(dtype=float)))


def volatility(returns: pd.Series) -> float:
    """Sample standard deviation (ddof=1) of the per-period returns."""
    values = returns.to_numpy(dtype=float)
    if len(values) < 2:
        raise ValueError("volatility needs at least two returns")
    return float(np.std(values, ddof=1))


def excess_returns(returns: pd.Series, cash_rate: float | pd.Series = 0.0) -> pd.Series:
    """Returns minus the per-period cash rate (a constant or a series on the same index)."""
    if isinstance(cash_rate, pd.Series):
        aligned = cash_rate.reindex(returns.index)
        if aligned.isna().any():
            raise ValueError("cash rate series does not cover every return date")
        return returns - aligned.astype(float)
    return returns - float(cash_rate)


def sharpe_ratio(
    returns: pd.Series,
    cash_rate: float | pd.Series = 0.0,
    periods_per_year: int = DEFAULT_PERIODS_PER_YEAR,
) -> float:
    """Annualized Sharpe ratio of the returns in excess of the cash rate.

    The cash rate is zero by default and is a per-period rate (a float or a series indexed like
    the returns). Annualization multiplies by the square root of ``periods_per_year``.
    """
    excess = excess_returns(returns, cash_rate)
    vol = volatility(excess)
    if vol == 0.0:
        raise ValueError("Sharpe ratio is undefined for zero volatility")
    return float(mean_return(excess) / vol * np.sqrt(periods_per_year))


def max_drawdown(returns: pd.Series) -> float:
    """Largest peak-to-trough fall of compounded equity, as a positive fraction.

    Equity starts at 1, and that starting value counts as a peak.
    """
    equity = np.concatenate([[1.0], np.cumprod(1.0 + returns.to_numpy(dtype=float))])
    peaks = np.maximum.accumulate(equity)
    return float(np.max(1.0 - equity / peaks))


def turnover(trades: pd.DataFrame) -> float:
    """Mean per-period turnover: the sum of buys as a fraction of portfolio value.

    ``trades`` holds traded weights (rows are periods, columns are assets). With a
    self-financing portfolio buys equal sells plus costs, so buys measure one-way turnover.
    """
    buys = trades.clip(lower=0.0).sum(axis=1)
    return float(buys.mean())


def leverage(weights: pd.DataFrame) -> float:
    """Mean gross leverage: the mean over periods of the sum of absolute weights."""
    return float(weights.abs().sum(axis=1).mean())


def total_cost(costs: pd.Series | pd.DataFrame) -> float:
    """Sum of all costs (a series per period, or a frame with one column per component)."""
    return float(np.sum(costs.to_numpy(dtype=float)))


@dataclass(frozen=True)
class PerformanceReport:
    """The metrics of R-197, with the cash rate and annualization they were computed under."""

    mean_return: float
    volatility: float
    sharpe_ratio: float
    max_drawdown: float
    turnover: float
    leverage: float
    total_cost: float
    cash_rate: str
    periods_per_year: int

    def to_dict(self) -> Mapping[str, float | str | int]:
        return asdict(self)


def compute_metrics(
    returns: pd.Series,
    trades: pd.DataFrame,
    weights: pd.DataFrame,
    costs: pd.Series | pd.DataFrame,
    cash_rate: float | pd.Series = 0.0,
    periods_per_year: int = DEFAULT_PERIODS_PER_YEAR,
) -> PerformanceReport:
    """Compute every metric. ``cash_rate`` in the report states which rate was used (R-198)."""
    description = "series" if isinstance(cash_rate, pd.Series) else f"constant {float(cash_rate)!r}"
    return PerformanceReport(
        mean_return=mean_return(returns),
        volatility=volatility(returns),
        sharpe_ratio=sharpe_ratio(returns, cash_rate, periods_per_year),
        max_drawdown=max_drawdown(returns),
        turnover=turnover(trades),
        leverage=leverage(weights),
        total_cost=total_cost(costs),
        cash_rate=description,
        periods_per_year=periods_per_year,
    )
