"""Factor attribution by ordinary least squares (R-209)."""

from __future__ import annotations

from dataclasses import dataclass

import numpy as np
import pandas as pd


@dataclass(frozen=True)
class Attribution:
    """Result of regressing returns on factor returns."""

    alpha: float
    alpha_t: float
    betas: pd.Series
    r_squared: float


def attribute(returns: pd.Series, factor_returns: pd.DataFrame) -> Attribution:
    """Regress ``returns`` on ``factor_returns`` with an intercept, by OLS.

    Rows are aligned on the index; rows with a missing value are dropped.
    """
    data = pd.concat([returns.rename("__y__"), factor_returns], axis=1, join="inner").dropna()
    y = data["__y__"].to_numpy(dtype=float)
    factors = data.drop(columns="__y__")
    x = np.column_stack([np.ones(len(data)), factors.to_numpy(dtype=float)])
    n, k = x.shape
    if n <= k:
        raise ValueError(f"need more than {k} observations, got {n}")

    coef, _, rank, _ = np.linalg.lstsq(x, y, rcond=None)
    if rank < k:
        raise ValueError("factor returns are collinear")
    resid = y - x @ coef
    ss_res = float(resid @ resid)
    sigma2 = ss_res / (n - k)
    cov = sigma2 * np.linalg.inv(x.T @ x)
    se_alpha = float(np.sqrt(cov[0, 0]))
    if se_alpha > 0:
        alpha_t = float(coef[0]) / se_alpha
    else:
        alpha_t = 0.0 if coef[0] == 0 else float(np.copysign(np.inf, coef[0]))
    ss_tot = float(((y - y.mean()) ** 2).sum())
    r_squared = 1.0 - ss_res / ss_tot if ss_tot > 0 else 0.0
    return Attribution(
        alpha=float(coef[0]),
        alpha_t=alpha_t,
        betas=pd.Series(coef[1:], index=factors.columns, dtype=float),
        r_squared=r_squared,
    )
