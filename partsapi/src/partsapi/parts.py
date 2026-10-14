"""Cross-vendor helpers. Prices are reported side by side, never silently ranked: JLCPCB/LCSC prices are
net USD (or the currency you set), Reichelt's are EUR gross incl. VAT."""
from __future__ import annotations

from typing import Any, Optional

from . import jlc, lcsc, reichelt
from .errors import PartsApiError
from .models import Part

VENDORS = {"jlc": jlc, "lcsc": lcsc, "reichelt": reichelt}


def findpart(query: str, *, limit: int = 5, vendors: tuple[str, ...] = ("jlc", "lcsc", "reichelt")) -> dict[str, list[Part]]:
    """Search several vendors; a vendor that errors yields an empty list (the error is in ``errors``)."""
    out: dict[str, list[Part]] = {}
    for v in vendors:
        try:
            out[v] = VENDORS[v].findpart(query, limit=limit)
        except PartsApiError:
            out[v] = []
    return out


def prices(mpn_or_code: str, qty: int = 1, *, vendors: tuple[str, ...] = ("jlc", "lcsc", "reichelt")) -> list[dict[str, Any]]:
    """Best-matching part per vendor and its unit price at ``qty``.

    A C-number is looked up directly at JLCPCB/LCSC; otherwise the first search hit whose MPN or article
    number equals the query (ignoring case/spaces/dashes) is used.
    """
    norm = lambda s: "".join(ch for ch in (s or "").upper() if ch.isalnum())  # noqa: E731
    rows: list[dict[str, Any]] = []
    for v in vendors:
        try:
            hits = VENDORS[v].findpart(mpn_or_code, limit=10)
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
