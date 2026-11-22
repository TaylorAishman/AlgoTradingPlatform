import numpy as np
import pandas as pd
import pytest

from atp.validation.attribution import attribute


def _data(seed: int = 7, n: int = 250) -> tuple[pd.Series, pd.DataFrame]:
    rng = np.random.default_rng(seed)
    idx = pd.bdate_range("2020-01-01", periods=n)
    f = pd.DataFrame(rng.normal(0, 0.01, (n, 4)), index=idx, columns=["mkt", "smb", "hml", "mom"])
    y = 0.0004 + f.to_numpy() @ np.array([1.1, 0.3, -0.2, 0.5]) + rng.normal(0, 0.005, n)
    return pd.Series(y, index=idx), f


def test_matches_least_squares() -> None:
    y, f = _data()
    res = attribute(y, f)

    x = np.column_stack([np.ones(len(y)), f.to_numpy()])
    coef = np.linalg.lstsq(x, y.to_numpy(), rcond=None)[0]
    resid = y.to_numpy() - x @ coef
    n, k = x.shape
    cov = resid @ resid / (n - k) * np.linalg.inv(x.T @ x)
    r2 = 1 - resid @ resid / ((y.to_numpy() - y.mean()) ** 2).sum()

    assert res.alpha == pytest.approx(coef[0], abs=1e-10)
    assert res.alpha_t == pytest.approx(coef[0] / np.sqrt(cov[0, 0]), abs=1e-10)
    assert list(res.betas.index) == list(f.columns)
    np.testing.assert_allclose(res.betas.to_numpy(), coef[1:], atol=1e-10, rtol=0)
    assert res.r_squared == pytest.approx(r2, abs=1e-10)


def test_aligns_on_index_and_drops_missing() -> None:
    y, f = _data()
    y2 = y.iloc[5:].copy()
    y2.iloc[3] = np.nan
    res = attribute(y2, f)
    keep = y2.dropna()
    ref = attribute(keep, f.loc[keep.index])
    assert res.alpha == pytest.approx(ref.alpha, abs=1e-12)


def test_rejects_too_few_or_collinear() -> None:
    y, f = _data(n=4)
    with pytest.raises(ValueError):
        attribute(y, f)
    y, f = _data()
    f["dup"] = f["mkt"]
    with pytest.raises(ValueError):
        attribute(y, f)
