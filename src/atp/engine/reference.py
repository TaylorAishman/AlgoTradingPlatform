"""Reference backtest after Algorithm 1 of *Implementation Risk in Portfolio Backtesting*.

This module imports only numpy and pandas so that it can serve as an independent test oracle.

Conventions (formulas.md IR-1 and IR-2):

- Every row of ``weights`` is a rebalance date; there are no non-rebalance days.
- Cost-then-allocate: the cost is computed from the trade deltas at the pre-cost value and
  deducted from the total portfolio value before reallocation.
- Fractional shares; the cost rate is a fraction of the traded dollar amount.
- The equity for a date is the value after that date's cost.
"""

import numpy as np
import pandas as pd


def reference_backtest(
    weights: pd.DataFrame, prices: pd.DataFrame, capital: float, cost_rate: float
) -> pd.Series:
    """Return the equity series of Algorithm 1 for target ``weights`` and close ``prices``."""
    if capital <= 0:
        raise ValueError("capital must be positive")
    if cost_rate < 0:
        raise ValueError("cost_rate must be non-negative")
    if weights.shape != prices.shape:
        raise ValueError("weights and prices must have the same shape")

    W = weights.to_numpy(dtype=float)
    P = prices.to_numpy(dtype=float)
    T, N = P.shape

    h = np.zeros(N)  # line 1: share holdings
    cash = float(capital)  # line 2
    equity = np.empty(T)
    for t in range(T):  # line 3
        p_t = P[t]
        V = cash + h @ p_t  # line 4: mark to market
        w = W[t]  # line 5: target weights
        delta = w * V - h * p_t  # line 6: trade deltas in dollars
        cost = cost_rate * np.abs(delta).sum()  # line 7
        V_net = V - cost  # line 8
        h = (w * V_net) / p_t  # line 9
        cash = V_net - h @ p_t  # line 10
        equity[t] = V_net  # line 11
    return pd.Series(equity, index=prices.index, name="equity")  # line 13
