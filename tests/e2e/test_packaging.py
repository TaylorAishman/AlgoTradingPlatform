from __future__ import annotations

import re
import subprocess
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
PYPROJECT = ROOT / "pyproject.toml"


def _names(requirements: list[str]) -> set[str]:
    return {re.split(r"[\s<>=!~\[;]", r, maxsplit=1)[0].lower() for r in requirements}


def test_dependency_allow_list() -> None:
    project = tomllib.loads(PYPROJECT.read_text())["project"]
    assert _names(project["dependencies"]) == {
        "numpy",
        "pandas",
        "pyarrow",
        "scipy",
        "scikit-learn",
        "cvxpy",
        "alpaca-py",
        "pydantic",
    }
    assert _names(project["optional-dependencies"]["dev"]) == {
        "pytest",
        "hypothesis",
        "ruff",
        "mypy",
    }
    assert project["requires-python"] == ">=3.11"
    assert project["scripts"]["atp"] == "atp.cli.main:main"


def test_default_markers_exclude_alpaca_and_slow(tmp_path: Path) -> None:
    (tmp_path / "test_marked.py").write_text(
        "import pytest\n"
        "def test_plain(): pass\n"
        "@pytest.mark.alpaca\n"
        "def test_alpaca(): pass\n"
        "@pytest.mark.slow\n"
        "def test_slow(): pass\n"
    )
    result = subprocess.run(
        [
            sys.executable,
            "-m",
            "pytest",
            "-q",
            "-c",
            str(PYPROJECT),
            "--rootdir",
            str(tmp_path),
            "-p",
            "no:cacheprovider",
            str(tmp_path / "test_marked.py"),
        ],
        capture_output=True,
        text=True,
        cwd=tmp_path,
        check=False,
    )
    assert result.returncode == 0, result.stdout + result.stderr
    assert "1 passed" in result.stdout
    assert "2 deselected" in result.stdout
