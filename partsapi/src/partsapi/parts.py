"""Cross-vendor helpers. Prices are reported side by side, never silently ranked: JLCPCB/LCSC prices are
net USD (or the currency you set), Reichelt's are EUR gross incl. VAT."""
from __future__ import annotations

import warnings
from typing import Any, Optional

from . import adafruit, digikey, jlc, lcsc, mouser, reichelt, sparkfun, tme
from .errors import PartsApiError
from .models import Part

VENDORS = {"jlc": jlc, "lcsc": lcsc, "reichelt": reichelt, "digikey": digikey, "mouser": mouser, "tme": tme, "adafruit": adafruit, "sparkfun": sparkfun}
KEYLESS = ("jlc", "lcsc", "reichelt")


def default_vendors() -> tuple[str, ...]:
    """JLCPCB, LCSC and Reichelt always; DigiKey / Mouser / TME only when their credentials are configured. Adafruit and
    SparkFun (hobby shops with their own part numbers, so rarely a match for an MPN) are only used when you name them."""
    return KEYLESS + tuple(v for v in ("digikey", "mouser", "tme") if VENDORS[v].is_configured())


def findpart(query: str, *, limit: int = 5, vendors: Optional[tuple[str, ...]] = None,
             where: Any = None, sort: Any = None, reverse: bool = False, qty: int = 1, **kw: Any) -> dict[str, list[Part]]:
    """Run the same query (full filter syntax) on several vendors (default: see :func:`default_vendors`).

    A vendor that cannot honour the query (error, or a filter that does not exist there such as
    ``is:preferred`` on Reichelt) contributes an empty list and a ``UserWarning`` saying why.
    """
    out: dict[str, list[Part]] = {}
    for v in vendors or default_vendors():
        try:
            out[v] = VENDORS[v].findpart(query, limit=limit, where=where, sort=sort, reverse=reverse, qty=qty, **kw)
        except (PartsApiError, ValueError) as exc:
            warnings.warn(f"{v}: {exc}", stacklevel=2)
            out[v] = []
    return out


def prices(mpn_or_code: str, qty: int = 1, *, vendors: Optional[tuple[str, ...]] = None) -> list[dict[str, Any]]:
    """Best-matching part per vendor and its unit price at ``qty``.

    A C-number is looked up directly at JLCPCB/LCSC; otherwise the first search hit whose MPN or vendor code
    equals the query (ignoring case/spaces/dashes) is used. DigiKey and Mouser are included only if configured.
    """
    norm = lambda s: "".join(ch for ch in (s or "").upper() if ch.isalnum())  # noqa: E731
    rows: list[dict[str, Any]] = []
    for v in vendors or default_vendors():
        try:
            hits = VENDORS[v].findpart(mpn_or_code, limit=30, parse=False)
        except PartsApiError as exc:
            rows.append({"vendor": v, "error": str(exc)})
            continue
        match: Optional[Part] = next((p for p in hits if norm(mpn_or_code) in (norm(p.code), norm(p.mpn or ""))), None)
        if match is None:
            rows.append({"vendor": v, "error": "no exact match", "candidates": [p.code for p in hits[:5]]})
            continue
        br = match.price_breaks[0] if match.price_breaks else None
        rows.append({
            "vendor": v, "code": match.code, "mpn": match.mpn, "unit_price": match.price_for(qty),
            "currency": br.currency if br else None, "incl_vat": br.incl_vat if br else None,
            "in_stock": match.stock.in_stock if match.stock else None,
            "stock_qty": match.stock.quantity if match.stock else None, "url": match.url,
        })
    return rows
