import dataclasses
from datetime import UTC, date, datetime, timedelta, timezone
from decimal import Decimal
from typing import Any

import pandas as pd
import pytest

from atp.core import errors
from atp.core.types import (
    AccountSnapshot,
    AssetId,
    Bar,
    Fill,
    Forecast,
    NewsItem,
    OrderRequest,
    Position,
    TargetPortfolio,
    build_client_order_id,
)

T = datetime(2024, 1, 2, 21, 0, tzinfo=UTC)
A = AssetId("A1")
S = pd.Series([0.1], index=[A])

FIELDS: dict[type, list[str]] = {
    Bar: [
        "asset", "session", "start", "end", "frequency", "open", "high", "low", "close",
        "volume", "vwap", "trade_count", "adjustment", "available_at",
    ],
    NewsItem: ["news_id", "assets", "headline", "summary", "source", "created_at", "available_at"],
    Forecast: ["as_of", "horizon_sessions", "mu", "rho", "sources"],
    TargetPortfolio: ["as_of", "weights", "cash_weight", "status", "diagnostics"],
    OrderRequest: [
        "client_order_id", "asset", "symbol", "side", "qty", "type", "time_in_force",
        "limit_price", "decision_as_of",
    ],
    Fill: ["client_order_id", "asset", "side", "qty", "price", "filled_at", "fees"],
    Position: ["asset", "qty", "market_value"],
    AccountSnapshot: ["as_of", "equity", "cash", "buying_power"],
}  # fmt: skip


def make(cls: type, t: datetime = T) -> Any:
    d = Decimal(1)
    kw: dict[type, dict[str, Any]] = {
        Bar: dict(
            asset=A, session=date(2024, 1, 2), start=t, end=t, frequency="1Day", open=1.0,
            high=1.0, low=1.0, close=1.0, volume=1.0, vwap=1.0, trade_count=1,
            adjustment="raw", available_at=t,
        ),
        NewsItem: dict(
            news_id="n", assets=(A,), headline="h", summary="s", source="x",
            created_at=t, available_at=t,
        ),
        Forecast: dict(as_of=t, horizon_sessions=1, mu=S, rho=S, sources={"a": 1.0}),
        TargetPortfolio: dict(
            as_of=t, weights=S, cash_weight=0.0, status="optimal", diagnostics={}
        ),
        OrderRequest: dict(
            client_order_id="c", asset=A, symbol="AAA", side="buy", qty=d, type="market",
            time_in_force="day", limit_price=None, decision_as_of=t,
        ),
        Fill: dict(client_order_id="c", asset=A, side="buy", qty=d, price=d, filled_at=t, fees=d),
        Position: dict(asset=A, qty=d, market_value=d),
        AccountSnapshot: dict(as_of=t, equity=d, cash=d, buying_power=d),
    }  # fmt: skip
    return cls(**kw[cls])


def order(**over: Any) -> OrderRequest:
    kw: dict[str, Any] = dict(
        client_order_id="c", asset=A, symbol="AAA", side="buy", qty=Decimal(1), type="market",
        time_in_force="day", limit_price=None, decision_as_of=T,
    )  # fmt: skip
    kw.update(over)
    return OrderRequest(**kw)


def test_fields_and_frozen() -> None:
    for cls, names in FIELDS.items():
        assert dataclasses.is_dataclass(cls)
        assert [f.name for f in dataclasses.fields(cls)] == names
        obj = make(cls)
        with pytest.raises(dataclasses.FrozenInstanceError):
            setattr(obj, names[0], None)
    assert issubclass(errors.NoForecastError, Exception)


def test_naive_datetime_rejected() -> None:
    naive = datetime(2024, 1, 2, 21, 0)
    eastern = timezone(timedelta(hours=-5))
    for cls in FIELDS:
        make(cls)  # UTC is accepted
        if cls is Position:  # no datetime field
            continue
        for bad in (naive, datetime(2024, 1, 2, 16, 0, tzinfo=eastern)):
            with pytest.raises(ValueError):
                make(cls, bad)


def test_client_order_id_format_and_length() -> None:
    day = date(2024, 1, 2)
    cid = build_client_order_id("mom", day, A, "buy", "0")
    assert cid == "mom-2024-01-02-A1-buy-0"
    pad = 128 - len(cid)
    assert len(build_client_order_id("m" * (3 + pad), day, A, "buy", "0")) == 128
    with pytest.raises(ValueError):
        build_client_order_id("m" * (4 + pad), day, A, "buy", "0")


def test_auction_order_requires_whole_shares() -> None:
    for tif in ("opg", "cls"):
        with pytest.raises(ValueError):
            order(time_in_force=tif, qty=Decimal("1.5"))
        order(time_in_force=tif, qty=Decimal("2"))
        order(time_in_force=tif, qty=Decimal("2.0"))
    order(time_in_force="day", qty=Decimal("1.5"))


def test_order_request_validation() -> None:
    for q in (Decimal(0), Decimal(-1), Decimal("NaN")):
        with pytest.raises(ValueError):
            order(qty=q)
    with pytest.raises(ValueError):
        order(type="limit", limit_price=None)
    order(type="limit", limit_price=Decimal("10"))
