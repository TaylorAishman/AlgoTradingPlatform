"""Root fixtures: offline guard and a temporary ATP_HOME."""

from __future__ import annotations

import socket
from collections.abc import Iterator
from pathlib import Path
from typing import Any

import pytest


@pytest.fixture(autouse=True)
def _block_network(
    request: pytest.FixtureRequest, monkeypatch: pytest.MonkeyPatch
) -> Iterator[None]:
    """Make any network connect raise, except in tests marked ``alpaca``."""
    if request.node.get_closest_marker("alpaca") is not None:
        yield
        return

    original = socket.socket.connect

    def refuse(self: socket.socket, *args: Any, **kwargs: Any) -> Any:
        if self.family == socket.AF_UNIX:
            return original(self, *args, **kwargs)
        raise RuntimeError("network access is blocked in offline tests")

    monkeypatch.setattr(socket.socket, "connect", refuse)
    monkeypatch.setattr(socket.socket, "connect_ex", refuse)
    yield


@pytest.fixture(autouse=True)
def atp_home(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> Path:
    """Point ATP_HOME at a fresh temporary directory for every test."""
    home = tmp_path / "atp_home"
    home.mkdir()
    monkeypatch.setenv("ATP_HOME", str(home))
    return home
