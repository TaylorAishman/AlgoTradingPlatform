"""Reserved fictitious symbols and company names for fixtures and the generator (R-282).

Every symbol starts with ``ZQ`` so that it cannot be mistaken for a listed security.
Fixtures and the synthetic generator draw only from this list.
"""

from __future__ import annotations

RESERVED_PREFIX = "ZQ"

COMPANY_NAMES: dict[str, str] = {
    "ZQAA": "Zephyr Quill Aerospace Holdings",
    "ZQAB": "Quillon Abbey Brewing Company",
    "ZQAC": "Zorvane Acoustic Systems",
    "ZQAD": "Quenby Adaptive Devices",
    "ZQAE": "Zalmora Energy Partners",
    "ZQAF": "Quorra Freight Lines",
    "ZQAG": "Zintel Agricultural Group",
    "ZQAH": "Quasar Harbor Logistics",
    "ZQAI": "Zubrin Instruments",
    "ZQAJ": "Quimby Jewelers Incorporated",
    "ZQAK": "Zakros Kinetics",
    "ZQAL": "Quallis Laboratories",
    "ZQAM": "Zomber Mining Corporation",
    "ZQAN": "Quinlan Networks",
    "ZQAO": "Zorathi Optics",
    "ZQAP": "Quennel Pharmaceuticals",
    "ZQAQ": "Zyloq Quarry Materials",
    "ZQAR": "Quartzel Realty Trust",
    "ZQAS": "Zeltic Semiconductor",
    "ZQAT": "Quibral Textiles",
    "ZQAU": "Zunder Utilities",
    "ZQAV": "Quovex Ventures",
    "ZQAW": "Zandry Water Works",
    "ZQAX": "Quixel Exploration",
    "ZQAY": "Zymber Yachts",
    "ZQAZ": "Quazio Zinc Holdings",
}

SYMBOLS: tuple[str, ...] = tuple(sorted(COMPANY_NAMES))


def is_reserved(symbol: str) -> bool:
    """Return True if the symbol is on the reserved fictitious list."""
    return symbol in COMPANY_NAMES
