"""Probability of backtest overfitting by combinatorially symmetric cross-validation.

Implements Algorithm 2.3 of *The Probability of Backtest Overfitting* (Bailey, Borwein,
Lopez de Prado and Zhu), as transcribed in ``docs/design/trading-platform/formulas.md``
(PBO-1 to PBO-6), for requirements R-203 to R-206.

The rows of the returns matrix are sessions and the columns are trials. The rows are cut
into ``n_blocks`` (S) contiguous blocks of equal length. Every one of the C(S, S/2) ways of
choosing S/2 blocks is a split: the chosen blocks, in their original order, are the
in-sample (IS) half and the rest the out-of-sample (OOS) half. In each split the trial with
the highest IS Sharpe ratio is selected, its OOS rank among all trials is turned into a
relative rank ``w = rank / (N + 1)`` (rank 1 is the worst, rank N the best) and into the
logit ``ln(w / (1 - w))``.

Performance statistic: the per-session Sharpe ratio, mean over standard deviation with
divisor ``n - 1``, not annualized. Within a split every trial has the same number of rows,
so the ranks, the logits, PBO, the slope and the probability of loss do not depend on the
divisor or on annualization (PBO-2).

Conventions chosen where the paper and the requirements are silent (FQ-5, FQ-6):

* FQ-6, a logit of exactly zero: counted as overfit, as R-203 states. PBO is the share of
  splits with a logit zero or negative.
* FQ-5 (a), rows not a multiple of S: the earliest ``T mod S`` rows are dropped, so the
  most recent sessions are always used.
* FQ-5 (b), ties: when several trials share the highest IS Sharpe ratio the first column
  among them is selected. Out of sample a tie takes the lowest rank of its group, that is
  ``1 +`` the number of trials with a strictly lower OOS Sharpe ratio. This is the
  conservative reading: a tie never makes the selected trial look better than it is.
* FQ-5 (c), zero variance: a trial whose returns in one half have zero variance (for
  example one that held only cash) has a Sharpe ratio of 0 in that half, whatever its mean.

Degradation slope (PBO-4): the ordinary least-squares slope of the selected trial's OOS
Sharpe ratio on its IS Sharpe ratio over all splits; NaN when the IS Sharpe ratios of the
selected trials do not vary. Probability of loss: the share of splits in which the
selected trial's OOS Sharpe ratio is strictly negative.
"""

from __future__ import annotations

from dataclasses import dataclass
from itertools import combinations, islice
from math import comb

import numpy as np
import numpy.typing as npt
import pandas as pd

__all__ = ["PBOResult", "pbo_cscv"]

# Splits evaluated per vectorized batch; bounds memory for large S.
_CHUNK = 65_536
# Relative variance below which a half is treated as having zero variance (FQ-5 c).
_ZERO_VAR_RTOL = 1e-12


@dataclass(frozen=True)
class PBOResult:
    """Result of :func:`pbo_cscv`.

    Attributes:
        pbo: Share of splits whose logit is zero or negative (R-203).
        logits: One logit per split, in the order of ``itertools.combinations`` over the
            IS block indices.
        degradation_slope: OLS slope of OOS on IS Sharpe ratio of the selected trials.
        probability_of_loss: Share of splits in which the selected trial's OOS Sharpe
            ratio is negative.
    """

    pbo: float
    logits: npt.NDArray[np.float64]
    degradation_slope: float
    probability_of_loss: float


def _half_sharpe(
    mask: npt.NDArray[np.float64],
    block_sum: npt.NDArray[np.float64],
    block_sumsq: npt.NDArray[np.float64],
    n_rows: int,
    col_mean: npt.NDArray[np.float64],
) -> npt.NDArray[np.float64]:
    """Sharpe ratios (splits x trials) of the halves selected by ``mask`` (splits x blocks)."""
    total = mask @ block_sum
    total_sq = mask @ block_sumsq
    centred_mean = total / n_rows
    var = (total_sq - n_rows * centred_mean**2) / (n_rows - 1)
    mean_sq = total_sq / n_rows
    zero_var = var <= _ZERO_VAR_RTOL * mean_sq
    std = np.sqrt(np.where(zero_var, 1.0, var))
    sharpe = (centred_mean + col_mean) / std
    return np.where(zero_var, 0.0, sharpe)


def pbo_cscv(returns_matrix: pd.DataFrame, n_blocks: int = 16) -> PBOResult:
    """Estimate the probability of backtest overfitting over all C(S, S/2) splits.

    Args:
        returns_matrix: Sessions x trials returns, rows synchronous across trials.
        n_blocks: Number S of blocks; even and at least 2.

    Returns:
        A :class:`PBOResult`.

    Raises:
        ValueError: If ``n_blocks`` is not an even number of at least 2, if there are fewer
            than 2 trials or fewer than ``2 * n_blocks`` rows, or if any value is not finite.
    """
    if isinstance(n_blocks, bool) or not isinstance(n_blocks, int | np.integer):
        raise ValueError(f"n_blocks must be an integer, got {n_blocks!r}")
    if n_blocks < 2 or n_blocks % 2 != 0:
        raise ValueError(f"n_blocks must be an even number of at least 2, got {n_blocks}")
    values = np.asarray(returns_matrix, dtype=np.float64)
    if values.ndim != 2:
        raise ValueError("returns_matrix must be two-dimensional (sessions x trials)")
    n_rows_total, n_trials = values.shape
    if n_trials < 2:
        raise ValueError(f"pbo_cscv needs at least 2 trials, got {n_trials}")
    if n_rows_total < 2 * n_blocks:
        raise ValueError(f"pbo_cscv needs at least 2 x S = {2 * n_blocks} rows, got {n_rows_total}")
    if not np.all(np.isfinite(values)):
        raise ValueError("returns_matrix must contain only finite values")

    block_len = n_rows_total // n_blocks
    values = values[n_rows_total - block_len * n_blocks :]  # FQ-5 (a): drop earliest rows
    col_mean = values.mean(axis=0)
    centred = values - col_mean  # centring keeps the sum-of-squares variance accurate
    blocks = centred.reshape(n_blocks, block_len, n_trials)
    block_sum = blocks.sum(axis=1)
    block_sumsq = (blocks**2).sum(axis=1)
    half_rows = block_len * n_blocks // 2

    half = n_blocks // 2
    n_splits = comb(n_blocks, half)
    combo_iter = combinations(range(n_blocks), half)

    logits = np.empty(n_splits, dtype=np.float64)
    is_selected = np.empty(n_splits, dtype=np.float64)
    oos_selected = np.empty(n_splits, dtype=np.float64)

    start = 0
    while start < n_splits:
        size = min(_CHUNK, n_splits - start)
        idx = np.array(list(islice(combo_iter, size)), dtype=np.intp).reshape(size, half)
        is_mask = np.zeros((size, n_blocks), dtype=np.float64)
        np.put_along_axis(is_mask, idx, 1.0, axis=1)
        oos_mask = 1.0 - is_mask

        sr_is = _half_sharpe(is_mask, block_sum, block_sumsq, half_rows, col_mean)
        sr_oos = _half_sharpe(oos_mask, block_sum, block_sumsq, half_rows, col_mean)

        best = np.argmax(sr_is, axis=1)  # FQ-5 (b): first column among IS ties
        rows = np.arange(size)
        sel_is = sr_is[rows, best]
        sel_oos = sr_oos[rows, best]
        rank = 1 + np.sum(sr_oos < sel_oos[:, None], axis=1)  # FQ-5 (b): lowest tied rank
        omega = rank / (n_trials + 1)

        stop = start + size
        logits[start:stop] = np.log(omega / (1.0 - omega))
        is_selected[start:stop] = sel_is
        oos_selected[start:stop] = sel_oos
        start = stop

    pbo = float(np.mean(logits <= 0.0))  # FQ-6: a zero logit counts as overfit
    probability_of_loss = float(np.mean(oos_selected < 0.0))

    x_dev = is_selected - is_selected.mean()
    sxx = float(np.dot(x_dev, x_dev))
    if sxx > 0.0:
        slope = float(np.dot(x_dev, oos_selected - oos_selected.mean()) / sxx)
    else:
        slope = float("nan")

    return PBOResult(
        pbo=pbo,
        logits=logits,
        degradation_slope=slope,
        probability_of_loss=probability_of_loss,
    )
