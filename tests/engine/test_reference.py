import ast
from pathlib import Path

import numpy as np
import pandas as pd

from atp.engine import reference
from atp.engine.reference import reference_backtest


def test_hand_computed_example() -> None:
    weights = pd.DataFrame([[0.5, 0.5], [1.0, 0.0]])
    prices = pd.DataFrame([[10.0, 20.0], [11.0, 20.0]])
    equity = reference_backtest(weights, prices, 1000.0, 0.001)
    np.testing.assert_allclose(equity.to_numpy(), [999.0, 1047.951], rtol=0, atol=1e-9)
    assert equity.index.equals(prices.index)


def test_zero_cost_example() -> None:
    weights = pd.DataFrame([[0.5, 0.5], [1.0, 0.0]])
    prices = pd.DataFrame([[10.0, 20.0], [11.0, 20.0]])
    equity = reference_backtest(weights, prices, 1000.0, 0.0)
    np.testing.assert_allclose(equity.to_numpy(), [1000.0, 1050.0], rtol=0, atol=1e-9)


def test_weights_below_one_leave_cash() -> None:
    weights = pd.DataFrame([[0.5], [0.5]])
    prices = pd.DataFrame([[10.0], [20.0]])
    equity = reference_backtest(weights, prices, 1000.0, 0.0)
    np.testing.assert_allclose(equity.to_numpy(), [1000.0, 1500.0], atol=1e-9)


def test_reference_is_independent() -> None:
    source = Path(reference.__file__).read_text()
    allowed = {"numpy", "pandas"}
    found: set[str] = set()
    for node in ast.walk(ast.parse(source)):
        if isinstance(node, ast.Import):
            found |= {a.name.split(".")[0] for a in node.names}
        elif isinstance(node, ast.ImportFrom):
            assert node.level == 0, "relative import"
            found.add((node.module or "").split(".")[0])
    assert found <= allowed, found - allowed
