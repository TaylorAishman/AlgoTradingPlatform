from __future__ import annotations

import pytest

from atp.cli.main import main

COMMANDS: list[tuple[list[str], list[str]]] = [
    (
        ["data", "ingest"],
        ["--dataset", "--start", "--end", "--symbols", "--feed", "quotes", "corporate-actions"],
    ),
    (["data", "check"], ["--start", "--end"]),
    (["universe", "build"], ["--config", "--start", "--end"]),
    (
        ["backtest", "run"],
        [
            "--config",
            "--start",
            "--end",
            "--family",
            "--synthetic",
            "--cost-scale",
            "--out",
        ],
    ),
    (["validate"], ["--family", "--select", "--gates", "--report"]),
    (["trials", "list"], ["--family", "--trial"]),
    (["trials", "show"], ["--family", "--trial"]),
    (["paper", "decide"], ["--config", "--session", "--dry-run", "--takeover"]),
    (["paper", "reconcile"], ["--session"]),
    (["paper", "status"], []),
    (["kill", "on"], ["--flatten", "--reason"]),
    (["kill", "off"], ["--flatten", "--reason"]),
    (["kill", "status"], ["--flatten", "--reason"]),
    (["report", "run"], ["--id", "--out"]),
    (["report", "paper"], ["--id", "--out"]),
]


def test_commands_and_flags(capsys: pytest.CaptureFixture[str]) -> None:
    for words, flags in COMMANDS:
        with pytest.raises(SystemExit) as exc:
            main([*words, "--help"])
        assert exc.value.code == 0, words
        out = capsys.readouterr().out
        for flag in flags:
            assert flag in out, (words, flag)


def test_usage_error_exits_2() -> None:
    with pytest.raises(SystemExit) as exc:
        main(["data", "ingest", "--dataset", "nope"])
    assert exc.value.code == 2
