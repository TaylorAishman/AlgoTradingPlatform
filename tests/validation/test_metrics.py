import math

import pandas as pd
import pytest

from atp.validation.metrics import compute_metrics, max_drawdown, sharpe_ratio

TOL = 1e-12

IDX = pd.date_range("2024-01-02", periods=4, freq="B")
RETURNS = pd.Series([0.10, -0.20, 0.05, 0.15], index=IDX)
TRADES = pd.DataFrame({"A": [0.5, -0.2, 0.1, 0.0], "B": [-0.1, 0.3, -0.4, 0.2]}, index=IDX)
WEIGHTS = pd.DataFrame({"A": [0.5, 0.3, 0.4, 0.4], "B": [-0.1, 0.2, -0.2, 0.0]}, index=IDX)
COSTS = pd.DataFrame(
    {"spread": [0.001, 0.002, 0.0, 0.003], "impact": [0.0, 0.001, 0.002, 0.0]}, index=IDX
)


def test_known_values() -> None:
    report = compute_metrics(RETURNS, TRADES, WEIGHTS, COSTS)
    # mean = 0.10 / 4; deviations 0.075, -0.225, 0.025, 0.125; squares sum 0.0725; var = 0.0725 / 3
    mean = 0.025
    vol = math.sqrt(0.0725 / 3)
    assert report.mean_return == pytest.approx(mean, abs=TOL)
    assert report.volatility == pytest.approx(vol, abs=TOL)
    assert report.sharpe_ratio == pytest.approx(mean / vol * math.sqrt(252), abs=TOL)
    # equity 1.1, 0.88, 0.924, 1.0626; peak 1.1; trough 0.88 -> 0.2
    assert report.max_drawdown == pytest.approx(0.2, abs=TOL)
    # buys per period: 0.5, 0.3, 0.1, 0.2
    assert report.turnover == pytest.approx(0.275, abs=TOL)
    # gross: 0.6, 0.5, 0.6, 0.4
    assert report.leverage == pytest.approx(0.525, abs=TOL)
    assert report.total_cost == pytest.approx(0.009, abs=TOL)


def test_max_drawdown_counts_initial_peak() -> None:
    assert max_drawdown(pd.Series([-0.1, -0.1])) == pytest.approx(0.19, abs=TOL)
    assert max_drawdown(pd.Series([0.1, 0.1])) == 0.0


def test_sharpe_excess_and_annualization() -> None:
    cash_const = 0.001
    excess = RETURNS - cash_const
    mean = float(excess.mean())
    vol = float(excess.std(ddof=1))
    assert sharpe_ratio(RETURNS, cash_const) == pytest.approx(mean / vol * math.sqrt(252), abs=TOL)

    # Zero is the default cash rate.
    assert sharpe_ratio(RETURNS) == sharpe_ratio(RETURNS, 0.0)
    assert sharpe_ratio(RETURNS) != sharpe_ratio(RETURNS, cash_const)

    # A series cash rate is subtracted date by date.
    cash = pd.Series([0.0, 0.001, 0.002, 0.003], index=IDX)
    ex = RETURNS - cash
    expected = float(ex.mean()) / float(ex.std(ddof=1)) * math.sqrt(252)
    assert sharpe_ratio(RETURNS, cash) == pytest.approx(expected, abs=TOL)

    # Annualization: default 252, configurable.
    base = sharpe_ratio(RETURNS, periods_per_year=1)
    assert sharpe_ratio(RETURNS) == pytest.approx(base * math.sqrt(252), abs=TOL)
    assert sharpe_ratio(RETURNS, periods_per_year=12) == pytest.approx(
        base * math.sqrt(12), abs=TOL
    )

    # The report states which cash rate was used.
    assert compute_metrics(RETURNS, TRADES, WEIGHTS, COSTS).cash_rate == "constant 0.0"
    assert compute_metrics(RETURNS, TRADES, WEIGHTS, COSTS, cash).cash_rate == "series"

    with pytest.raises(ValueError):
        sharpe_ratio(RETURNS, cash.iloc[:2])
