"""The ``ATP_HOME`` directory: where everything mutable is written (Arch 5.12).

Layout::

    $ATP_HOME/data/          Parquet datasets of Arch 5.3
    $ATP_HOME/models/        predictor artifacts, one directory per predictor and train_end
    $ATP_HOME/runs/<id>/     config snapshot, decisions, orders, fills, equity, metrics
    $ATP_HOME/llm_cache/     cached model responses
    $ATP_HOME/logs/          structured logs, one file per job run
    $ATP_HOME/bench/         benchmark records (requirements I29)
    $ATP_HOME/state.sqlite   operational state, see ``atp.core.state``

``ATP_HOME`` defaults to ``./var``, relative to the current working directory.
"""

from __future__ import annotations

import os
from collections.abc import Mapping
from dataclasses import dataclass
from pathlib import Path

ENV_VAR = "ATP_HOME"
DEFAULT_HOME = Path("var")
STATE_FILE = "state.sqlite"


def resolve_home(environ: Mapping[str, str] | None = None) -> Path:
    """Return the absolute ``ATP_HOME`` path; an unset or empty variable means ``./var``."""
    env = os.environ if environ is None else environ
    value = env.get(ENV_VAR, "").strip()
    path = Path(value).expanduser() if value else DEFAULT_HOME
    return path.resolve()


@dataclass(frozen=True)
class HomeLayout:
    """Paths of the ``ATP_HOME`` layout. Building one touches nothing on disk."""

    root: Path

    @property
    def data(self) -> Path:
        return self.root / "data"

    @property
    def models(self) -> Path:
        return self.root / "models"

    @property
    def runs(self) -> Path:
        return self.root / "runs"

    @property
    def llm_cache(self) -> Path:
        return self.root / "llm_cache"

    @property
    def logs(self) -> Path:
        return self.root / "logs"

    @property
    def bench(self) -> Path:
        return self.root / "bench"

    @property
    def state_db(self) -> Path:
        return self.root / STATE_FILE

    def directories(self) -> tuple[Path, ...]:
        """The directories of the layout, in the order of Arch 5.12."""
        return (self.data, self.models, self.runs, self.llm_cache, self.logs, self.bench)

    def run_dir(self, run_id: str) -> Path:
        """Directory of one run; ``run_id`` must be a single plain path component."""
        if (
            not run_id
            or run_id in {".", ".."}
            or "/" in run_id
            or "\\" in run_id
            or "\x00" in run_id
        ):
            raise ValueError(f"invalid run id: {run_id!r}")
        return self.runs / run_id


def home_layout(root: Path | None = None) -> HomeLayout:
    """Layout rooted at ``root``, or at the resolved ``ATP_HOME`` when ``root`` is None."""
    return HomeLayout(resolve_home() if root is None else Path(root).resolve())


def ensure_layout(root: Path | None = None) -> HomeLayout:
    """Create the layout's directories if missing and return the layout. Idempotent."""
    layout = home_layout(root)
    for directory in layout.directories():
        directory.mkdir(parents=True, exist_ok=True)
    return layout
