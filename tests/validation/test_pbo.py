"""Tests for ``atp.validation.pbo`` (R-203 to R-206; formulas.md PBO-1 to PBO-6).

Conventions under test where the paper is silent (FQ-5, FQ-6), as stated in the module
docstring: a logit of exactly zero counts as overfit; the earliest ``T mod S`` rows are
dropped; IS ties select the first column and OOS ties take the lowest rank of their group;
a half with zero variance has a Sharpe ratio of 0.
"""

from __future__ import annotations

import math
from dataclasses import FrozenInstanceError

import numpy as np
import pandas as pd
import pytest

from atp.validation.pbo import PBOResult, pbo_cscv

# Worked example PBO-6: T = 8, N = 3, S = 4.
WORKED_EXAMPLE = pd.DataFrame(
    [
        [3, -1, -1],
        [1, 0, 0],
        [3, 3, -2],
        [0, 1, 0],
        [3, -1, 3],
        [-1, 3, 3],
        [-1, -1, -2],
        [3, -2, 1],
    ],
    columns=["trial_1", "trial_2", "trial_3"],
    dtype=float,
)

NOISE_SEEDS = (0, 1, 2, 3, 4)
N_TRIALS = 50
N_SESSIONS = 1024


def _noise_family(seed: int) -> pd.DataFrame:
    rng = np.random.default_rng(seed)
    return pd.DataFrame(rng.standard_normal((N_SESSIONS, N_TRIALS)))


def test_split_count_and_definition() -> None:
    result = pbo_cscv(WORKED_EXAMPLE, n_blocks=4)

    # Splits in itertools.combinations order of IS blocks: (1,2) (1,3) (1,4) (2,3) (2,4) (3,4).
    ln3 = math.log(3.0)
    expected_logits = [0.0, ln3, 0.0, -ln3, ln3, -ln3]
    np.testing.assert_allclose(result.logits, expected_logits, atol=1e-12)

    # FQ-6: zero logits count as overfit (4/6); the strict reading would give 2/6.
    assert result.pbo == pytest.approx(4 / 6, abs=1e-12)
    assert result.degradation_slope == pytest.approx(0.947884, abs=1e-6)
    assert result.probability_of_loss == pytest.approx(2 / 6, abs=1e-12)

    # Ranking by Sharpe ratio does not depend on scale (annualization or divisor).
    scaled = pbo_cscv(WORKED_EXAMPLE * 7.5, n_blocks=4)
    np.testing.assert_allclose(scaled.logits, result.logits, atol=1e-12)

    # Count: all C(S, S/2) splits; 12,870 for S = 16 (the paper misprints 12,780).
    rng = np.random.default_rng(11)
    small = pd.DataFrame(rng.standard_normal((64, 4)))
    assert pbo_cscv(small, n_blocks=16).logits.shape == (12_870,)
    assert pbo_cscv(small, n_blocks=12).logits.shape == (924,)
    assert pbo_cscv(small, n_blocks=2).logits.shape == (2,)


def test_conventions_for_open_questions() -> None:
    # FQ-5 (a): rows beyond a multiple of S are dropped from the start.
    rng = np.random.default_rng(5)
    extra = pd.DataFrame(rng.standard_normal((3, 3)) * 100.0, columns=WORKED_EXAMPLE.columns)
    padded = pd.concat([extra, WORKED_EXAMPLE], ignore_index=True)
    np.testing.assert_allclose(
        pbo_cscv(padded, n_blocks=4).logits, pbo_cscv(WORKED_EXAMPLE, n_blocks=4).logits
    )

    # FQ-5 (b) and (c): two identical cash trials (zero variance, Sharpe 0) and one that
    # loses in every half. The first cash column is selected in sample; out of sample it
    # ties with the other cash trial and takes the lower rank, 2 of 3: logit 0, overfit.
    loser = np.tile([-1.0, -2.0], 4)
    tied = pd.DataFrame({"cash_a": np.zeros(8), "cash_b": np.zeros(8), "loser": loser})
    result = pbo_cscv(tied, n_blocks=4)
    np.testing.assert_allclose(result.logits, np.zeros(6), atol=1e-12)
    assert result.pbo == 1.0
    assert result.probability_of_loss == 0.0
    assert math.isnan(result.degradation_slope)  # selected IS Sharpe ratio never varies


@pytest.mark.parametrize("seed", NOISE_SEEDS)
def test_noise_family_is_overfit(seed: int) -> None:
    result = pbo_cscv(_noise_family(seed), n_blocks=16)
    assert result.logits.shape == (12_870,)
    assert result.pbo >= 0.2


@pytest.mark.parametrize("seed", NOISE_SEEDS)
def test_planted_signal_is_not_overfit(seed: int) -> None:
    family = _noise_family(seed)
    family[17] += 0.3  # mean of 0.3 standard deviations per session
    result = pbo_cscv(family, n_blocks=16)
    assert result.pbo <= 0.05


def test_result_fields_and_input_validation() -> None:
    result = pbo_cscv(WORKED_EXAMPLE, n_blocks=4)
    assert isinstance(result, PBOResult)
    assert isinstance(result.pbo, float)
    assert isinstance(result.degradation_slope, float)
    assert isinstance(result.probability_of_loss, float)
    assert isinstance(result.logits, np.ndarray)
    assert result.logits.dtype == np.float64
    with pytest.raises(FrozenInstanceError):
        result.pbo = 0.0  # type: ignore[misc]

    # Fewer than 2 trials.
    with pytest.raises(ValueError, match="2 trials"):
        pbo_cscv(WORKED_EXAMPLE[["trial_1"]], n_blocks=4)
    with pytest.raises(ValueError, match="2 trials"):
        pbo_cscv(pd.DataFrame(index=range(40)), n_blocks=4)

    # Fewer than 2 x S rows: 7 rows at S = 4 fails, 8 rows passes.
    with pytest.raises(ValueError, match="rows"):
        pbo_cscv(WORKED_EXAMPLE.iloc[:7], n_blocks=4)
    with pytest.raises(ValueError, match="rows"):
        pbo_cscv(_noise_family(0).iloc[:31], n_blocks=16)
    pbo_cscv(WORKED_EXAMPLE.iloc[:8], n_blocks=4)

    # S must be an even integer of at least 2.
    for bad in (0, 3, -2):
        with pytest.raises(ValueError, match="n_blocks"):
            pbo_cscv(WORKED_EXAMPLE, n_blocks=bad)

    # Non-finite values.
    with pytest.raises(ValueError, match="finite"):
        pbo_cscv(WORKED_EXAMPLE.replace(3.0, np.nan), n_blocks=4)
