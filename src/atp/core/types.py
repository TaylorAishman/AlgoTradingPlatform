"""Core value types of Arch 5.1."""

from __future__ import annotations

from collections.abc import Mapping
from dataclasses import dataclass, fields
from datetime import date, datetime, timedelta
from decimal import Decimal
from typing import Any, Literal, NewType

import pandas as pd

AssetId = NewType("AssetId", str)
Frequency = Literal["1Day", "1Min"]

MAX_CLIENT_ORDER_ID_LENGTH = 128


def _check_utc_datetimes(obj: Any) -> None:
    """Raise ValueError if any datetime field of a dataclass is naive or not UTC."""
    for f in fields(obj):
        value = getattr(obj, f.name)
        if isinstance(value, datetime):
            offset = value.utcoffset()
            if offset is None or offset != timedelta(0):
                raise ValueError(f"{type(obj).__name__}.{f.name} must be timezone-aware UTC")


def build_client_order_id(
    strategy: str, session: date, asset: AssetId | str, side: str, slice: str | int
) -> str:
    """Build the deterministic client order id; at most 128 characters."""
    value = f"{strategy}-{session}-{asset}-{side}-{slice}"
    if len(value) > MAX_CLIENT_ORDER_ID_LENGTH:
        raise ValueError(
            f"client_order_id is {len(value)} characters; the limit is {MAX_CLIENT_ORDER_ID_LENGTH}"
        )
    return value


@dataclass(frozen=True)
class Bar:
    asset: AssetId
    session: date
    start: datetime
    end: datetime
    frequency: Frequency
    open: float
    high: float
    low: float
    close: float
    volume: float
    vwap: float
    trade_count: int
    adjustment: Literal["raw", "split", "all"]
    available_at: datetime

    def __post_init__(self) -> None:
        _check_utc_datetimes(self)


@dataclass(frozen=True)
class NewsItem:
    news_id: str
    assets: tuple[AssetId, ...]
    headline: str
    summary: str
    source: str
    created_at: datetime
    available_at: datetime

    def __post_init__(self) -> None:
        _check_utc_datetimes(self)


@dataclass(frozen=True)
class Forecast:
    as_of: datetime
    horizon_sessions: int
    mu: pd.Series[Any]
    rho: pd.Series[Any]
    sources: Mapping[str, float]

    def __post_init__(self) -> None:
        _check_utc_datetimes(self)


@dataclass(frozen=True)
class TargetPortfolio:
    as_of: datetime
    weights: pd.Series[Any]
    cash_weight: float
    status: Literal["optimal", "relaxed", "fallback_hold"]
    diagnostics: Mapping[str, float]

    def __post_init__(self) -> None:
        _check_utc_datetimes(self)


@dataclass(frozen=True)
class OrderRequest:
    client_order_id: str
    asset: AssetId
    symbol: str
    side: Literal["buy", "sell"]
    qty: Decimal
    type: Literal["market", "limit"]
    time_in_force: Literal["opg", "cls", "day"]
    limit_price: Decimal | None
    decision_as_of: datetime

    def __post_init__(self) -> None:
        _check_utc_datetimes(self)
        if not self.qty.is_finite() or self.qty <= 0:
            raise ValueError("qty must be a finite number above zero")
        if self.type == "limit" and self.limit_price is None:
            raise ValueError("a limit order requires limit_price")
        if self.time_in_force in ("opg", "cls") and self.qty != self.qty.to_integral_value():
            raise ValueError(f"a {self.time_in_force} order requires a whole number of shares")


@dataclass(frozen=True)
class Fill:
    client_order_id: str
    asset: AssetId
    side: Literal["buy", "sell"]
    qty: Decimal
    price: Decimal
    filled_at: datetime
    fees: Decimal

    def __post_init__(self) -> None:
        _check_utc_datetimes(self)


@dataclass(frozen=True)
class Position:
    asset: AssetId
    qty: Decimal
    market_value: Decimal


@dataclass(frozen=True)
class AccountSnapshot:
    as_of: datetime
    equity: Decimal
    cash: Decimal
    buying_power: Decimal

    def __post_init__(self) -> None:
        _check_utc_datetimes(self)
