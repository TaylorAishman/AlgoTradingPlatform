from __future__ import annotations

import os
import socket
from pathlib import Path

import pytest


def test_socket_connect_blocked() -> None:
    s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    try:
        with pytest.raises(RuntimeError, match="network access is blocked"):
            s.connect(("127.0.0.1", 9))
    finally:
        s.close()


def test_atp_home_is_temporary(atp_home: Path) -> None:
    assert os.environ["ATP_HOME"] == str(atp_home)
    assert atp_home.is_dir()
