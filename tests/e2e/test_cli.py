from __future__ import annotations

import argparse
import sys
import types

import pytest

from atp.cli.main import build_parser, main

COMMANDS: list[tuple[list[str], list[str]]] = [
    (
        ["data", "ingest"],
        ["--dataset", "--start", "--end", "--symbols", "--feed"],
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


def _options(words: list[str]) -> set[str]:
    parser = build_parser()
    for word in words:
        subs = [a for a in parser._actions if isinstance(a, argparse._SubParsersAction)]
        parser = subs[0].choices[word]
    return {o for a in parser._actions for o in a.option_strings}


def test_commands_and_flags(capsys: pytest.CaptureFixture[str]) -> None:
    for words, flags in COMMANDS:
        with pytest.raises(SystemExit) as exc:
            main([*words, "--help"])
        assert exc.value.code == 0, words
        capsys.readouterr()
        assert set(flags) <= _options(words), words


def test_ingest_accepts_quotes_dataset() -> None:
    args = build_parser().parse_args(["data", "ingest", "--dataset", "quotes"])
    assert args.dataset == "quotes"


def test_usage_error_exits_2() -> None:
    with pytest.raises(SystemExit) as exc:
        main(["data", "ingest", "--dataset", "nope"])
    assert exc.value.code == 2


def _stub(monkeypatch: pytest.MonkeyPatch, module: str, run: object) -> None:
    mod = types.ModuleType(f"atp.cli.{module}")
    mod.run = run  # type: ignore[attr-defined]
    monkeypatch.setitem(sys.modules, f"atp.cli.{module}", mod)


@pytest.mark.parametrize("code", [0, 3, 4])
def test_run_return_value_is_exit_code(monkeypatch: pytest.MonkeyPatch, code: int) -> None:
    seen: list[argparse.Namespace] = []

    def run(args: argparse.Namespace) -> int:
        seen.append(args)
        return code

    _stub(monkeypatch, "paper_status", run)
    assert main(["paper", "status"]) == code
    assert len(seen) == 1


def test_missing_module_exits_1_with_message(
    monkeypatch: pytest.MonkeyPatch, capsys: pytest.CaptureFixture[str]
) -> None:
    import importlib

    real = importlib.import_module

    def fake(name: str, *a: object, **k: object) -> types.ModuleType:
        if name == "atp.cli.paper_status":
            raise ModuleNotFoundError(f"No module named {name!r}", name=name)
        return real(name, *a, **k)  # type: ignore[arg-type]

    monkeypatch.setattr(importlib, "import_module", fake)
    assert main(["paper", "status"]) == 1
    err = capsys.readouterr().err
    assert "not implemented" in err
    assert "atp.cli.paper_status" in err


def test_import_error_inside_module_is_reraised(monkeypatch: pytest.MonkeyPatch) -> None:
    import importlib

    def fake(name: str, *a: object, **k: object) -> types.ModuleType:
        raise ModuleNotFoundError("No module named 'dep'", name="dep")

    monkeypatch.setattr(importlib, "import_module", fake)
    with pytest.raises(ModuleNotFoundError, match="dep"):
        main(["paper", "status"])


# Module of every leaf command, as named by the plan (J04 to J21 file lists).
LEAF_MODULES: list[tuple[list[str], str]] = [
    (["data", "ingest", "--dataset", "bars"], "data_ingest"),
    (["data", "check"], "data_check"),
    (["universe", "build", "--config", "c"], "universe_build"),
    (["backtest", "run", "--config", "c"], "backtest_run"),
    (["validate", "--family", "f"], "validate"),
    (["trials", "list"], "trials"),
    (["trials", "show"], "trials"),
    (["paper", "decide", "--config", "c"], "paper_decide"),
    (["paper", "reconcile"], "paper_reconcile"),
    (["paper", "status"], "paper_status"),
    (["kill", "on"], "kill"),
    (["kill", "off"], "kill"),
    (["kill", "status"], "kill"),
    (["report", "run", "--id", "i"], "report_run"),
    (["report", "paper", "--id", "i"], "report_paper"),
]


def test_leaf_module_names_match_the_plan() -> None:
    assert len(LEAF_MODULES) == len(COMMANDS)
    for argv, module in LEAF_MODULES:
        assert build_parser().parse_args(argv).module == module, argv
