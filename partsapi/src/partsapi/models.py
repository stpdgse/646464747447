"""Plain dataclasses shared by every vendor module."""
from __future__ import annotations

from dataclasses import asdict, dataclass, field
from typing import Any, Optional


@dataclass
class PriceBreak:
    """One rung of a quantity price ladder. ``max_qty=None`` means "and above"."""

    min_qty: int
    max_qty: Optional[int]
    unit_price: float
    currency: str = "USD"
    incl_vat: Optional[bool] = None  # None = vendor does not say / not applicable
    vat_rate: Optional[float] = None  # e.g. 0.19

    def covers(self, qty: int) -> bool:
        return qty >= self.min_qty and (self.max_qty is None or qty <= self.max_qty)


@dataclass
class Stock:
    vendor: str
    code: str
    quantity: Optional[int]  # None when the vendor only says in/out of stock
    in_stock: bool
    delivery: Optional[str] = None  # free-text delivery estimate, if the vendor gives one
    detail: dict[str, Any] = field(default_factory=dict)  # vendor-specific breakdown


@dataclass
class Model3D:
    """A downloaded 3D model plus where it came from."""

    data: bytes = field(repr=False)
    format: str  # "step" or "obj"
    filename: str
    source_code: str  # the LCSC/EasyEDA part the model was resolved from
    uuid: str
    attribution: str = (
        "3D model from the JLCEDA/EasyEDA Official Library (https://lceda.cn/, https://easyeda.com)"
    )


@dataclass
class Part:
    """Everything known about a part. ``raw`` keeps the untouched vendor payload."""

    vendor: str  # "jlc" | "lcsc" | "reichelt"
    code: str  # C-number for jlc/lcsc, article number for reichelt
    mpn: Optional[str] = None
    manufacturer: Optional[str] = None
    description: Optional[str] = None
    category: Optional[str] = None
    package: Optional[str] = None
    url: Optional[str] = None
    datasheet_url: Optional[str] = None
    image_urls: list[str] = field(default_factory=list)
    price_breaks: list[PriceBreak] = field(default_factory=list)
    stock: Optional[Stock] = None
    attributes: dict[str, str] = field(default_factory=dict)
    extra: dict[str, Any] = field(default_factory=dict)  # vendor-specific scalars (library type, MOQ, ...)
    raw: dict[str, Any] = field(default_factory=dict, repr=False)

    def price_for(self, qty: int = 1) -> Optional[float]:
        """Unit price at ``qty``; below the first rung the first rung applies."""
        return pick_price(self.price_breaks, qty)

    def to_dict(self, include_raw: bool = False) -> dict[str, Any]:
        d = asdict(self)
        if not include_raw:
            d.pop("raw", None)
        return d


def pick_price(breaks: list[PriceBreak], qty: int = 1) -> Optional[float]:
    if not breaks:
        return None
    if qty < 1:
        raise ValueError("qty must be >= 1")
    ordered = sorted(breaks, key=lambda b: b.min_qty)
    chosen = ordered[0]
    for b in ordered:
        if qty >= b.min_qty:
            chosen = b
    return chosen.unit_price
