"""Entry point of the ``atp`` command (Arch 5.11).

The parser knows every command and flag. Each command is handled by one module,
``atp.cli.<module>``, that exposes ``run(args: argparse.Namespace) -> int``. The
module is imported only when its command runs; a command whose module does not
exist yet exits 1.

Exit codes: 0 success, 1 failure, 2 usage error, 3 a gate failed, 4 blocked by
the risk gate or kill switch.
"""

from __future__ import annotations

import argparse
import importlib
import sys
from collections.abc import Sequence

DATASETS = ("bars", "corporate-actions", "assets", "calendar", "news", "quotes")
FEEDS = ("sip", "iex")


def _dates(p: argparse.ArgumentParser) -> None:
    p.add_argument("--start", metavar="D")
    p.add_argument("--end", metavar="D")


def _leaf(
    sub: argparse._SubParsersAction[argparse.ArgumentParser], name: str, module: str
) -> argparse.ArgumentParser:
    p = sub.add_parser(name)
    p.set_defaults(module=module)
    return p


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(prog="atp", description="Algorithmic trading platform")
    top = parser.add_subparsers(dest="command", metavar="COMMAND", required=True)

    data = top.add_parser("data", help="ingest and check data")
    data_sub = data.add_subparsers(dest="subcommand", metavar="SUBCOMMAND", required=True)
    p = _leaf(data_sub, "ingest", "data_ingest")
    p.add_argument("--dataset", required=True, choices=DATASETS)
    _dates(p)
    p.add_argument("--symbols", metavar="S,..")
    p.add_argument("--feed", choices=FEEDS)
    p = _leaf(data_sub, "check", "data_check")
    _dates(p)

    universe = top.add_parser("universe", help="build the universe")
    universe_sub = universe.add_subparsers(dest="subcommand", metavar="SUBCOMMAND", required=True)
    p = _leaf(universe_sub, "build", "universe_build")
    p.add_argument("--config", required=True, metavar="PATH")
    _dates(p)

    backtest = top.add_parser("backtest", help="run backtests")
    backtest_sub = backtest.add_subparsers(dest="subcommand", metavar="SUBCOMMAND", required=True)
    p = _leaf(backtest_sub, "run", "backtest_run")
    p.add_argument("--config", required=True, metavar="PATH")
    _dates(p)
    p.add_argument("--family", metavar="NAME")
    p.add_argument("--synthetic", type=int, metavar="SEED")
    p.add_argument("--cost-scale", type=float, metavar="X")
    p.add_argument("--out", metavar="DIR")

    p = top.add_parser("validate", help="run the gates")
    p.set_defaults(module="validate")
    p.add_argument("--family", required=True, metavar="NAME")
    p.add_argument("--select", metavar="TRIAL_ID")
    p.add_argument("--gates", metavar="PATH")
    p.add_argument("--report", metavar="PATH")

    trials = top.add_parser("trials", help="inspect the trial registry")
    trials_sub = trials.add_subparsers(dest="subcommand", metavar="{list,show}", required=True)
    for name in ("list", "show"):
        p = _leaf(trials_sub, name, "trials")
        p.add_argument("--family", metavar="NAME")
        p.add_argument("--trial", metavar="TRIAL_ID")

    paper = top.add_parser("paper", help="paper trading jobs")
    paper_sub = paper.add_subparsers(dest="subcommand", metavar="SUBCOMMAND", required=True)
    p = _leaf(paper_sub, "decide", "paper_decide")
    p.add_argument("--config", required=True, metavar="PATH")
    p.add_argument("--session", metavar="D")
    p.add_argument("--dry-run", action="store_true")
    p.add_argument("--takeover", action="store_true")
    p = _leaf(paper_sub, "reconcile", "paper_reconcile")
    p.add_argument("--session", metavar="D")
    _leaf(paper_sub, "status", "paper_status")

    kill = top.add_parser("kill", help="the kill switch")
    kill_sub = kill.add_subparsers(dest="subcommand", metavar="{on,off,status}", required=True)
    for name in ("on", "off", "status"):
        p = _leaf(kill_sub, name, "kill")
        p.add_argument("--flatten", action="store_true")
        p.add_argument("--reason", metavar="TEXT")

    report = top.add_parser("report", help="write a report")
    report_sub = report.add_subparsers(dest="subcommand", metavar="{run,paper}", required=True)
    for name in ("run", "paper"):
        p = _leaf(report_sub, name, f"report_{name}")
        p.add_argument("--id", required=True, metavar="ID")
        p.add_argument("--out", metavar="PATH")

    return parser


def main(argv: Sequence[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    name = f"atp.cli.{args.module}"
    try:
        module = importlib.import_module(name)
    except ModuleNotFoundError as exc:
        if exc.name != name:
            raise
        print(f"atp: command not implemented: {name}", file=sys.stderr)
        return 1
    return int(module.run(args))


if __name__ == "__main__":
    raise SystemExit(main())
