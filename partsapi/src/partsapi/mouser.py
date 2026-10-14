"""Mouser Electronics (https://www.mouser.com) via the official Search API.

Needs a (free) Search API key from https://www.mouser.com/api-hub/ ::

    export MOUSER_API_KEY=xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx

    from partsapi import mouser
    mouser.findpart("lm358 is:instock is:rohs price:<0.5 -dip", sort="price", limit=10)
    mouser.getpartprice("595-LM358DR", qty=100)
    mouser.getstock("LM358DR")
    mouser.getpic("595-LM358DR", save_to="out/")

Written against Mouser's published OpenAPI schema (V1 keyword / part-number search). Mouser's API is deliberately
small: it can filter server-side only by RoHS and in-stock; every other filter (manufacturer, package, price,
attributes, ...) is checked here while paging through results (50 per request). Mouser rate-limits per key
(the default spacing of 2 s between calls keeps you under 30 calls/minute); each page is one call, so
``max_pages`` bounds how much a filtered search can spend. Prices are in your account's currency, as formatted
by Mouser; ``PriceBreak.currency`` carries the code Mouser reports.
"""
from __future__ import annotations

import os
import re
from pathlib import Path
from typing import Any, Callable, Optional, Union

from . import easyeda as _easyeda
from ._http import Http
from ._util import PathLike, ext_from_url, parse_price, safe_name, save_bytes, to_float, to_int
from .errors import AuthError, BlockedError, NotFoundError, PartsApiError
from .filters import (ParsedQuery, PartFilter, build_filter, facets as _facets, parse_query, reject_unsupported, sort_parts)
from .models import Model3D, Part, PriceBreak, Stock, pick_price

BASE = "https://api.mouser.com/api"
PAGE = 50  # API maximum per request
PKG_ATTRS = ("packagecase", "package", "case", "supplierdevicepackage")


def _squash(s: str) -> str:
    return re.sub(r"[^a-z0-9]+", "", s.casefold())


def _breaks(items: Optional[list[dict[str, Any]]], default_currency: str) -> list[PriceBreak]:
    rows = []
    for i in items or []:
        price = parse_price(i.get("Price"))
        if price is not None and to_int(i.get("Quantity")):
            rows.append((int(i["Quantity"]), price, i.get("Currency") or default_currency))
    rows.sort()
    return [PriceBreak(min_qty=q, max_qty=(rows[n + 1][0] - 1 if n + 1 < len(rows) else None), unit_price=p, currency=c)
            for n, (q, p, c) in enumerate(rows)]


def _stock_qty(p: dict[str, Any]) -> Optional[int]:
    q = to_int(re.sub(r"[^\d]", "", str(p.get("AvailabilityInStock") or "")) or None)
    if q is not None:
        return q
    m = re.match(r"\s*([\d,.]+)\s+In Stock", str(p.get("Availability") or ""), re.I)
    return to_int(re.sub(r"[^\d]", "", m.group(1))) if m else None


def _parse(p: dict[str, Any], currency: str = "USD") -> Part:
    attrs = {a["AttributeName"]: a.get("AttributeValue") for a in p.get("ProductAttributes") or [] if a.get("AttributeName")}
    package = next((v for k, v in attrs.items() if _squash(k) in PKG_ATTRS), None)
    qty = _stock_qty(p)
    rohs = str(p.get("ROHSStatus") or "")
    code = p.get("MouserPartNumber") or p.get("ManufacturerPartNumber") or ""
    on_order = [{"quantity": to_int(o.get("Quantity")), "date": o.get("Date")} for o in p.get("AvailabilityOnOrder") or []]
    images = [p["ImagePath"]] if p.get("ImagePath") else []
    return Part(
        vendor="mouser",
        code=code,
        mpn=p.get("ManufacturerPartNumber"),
        manufacturer=p.get("Manufacturer") or p.get("ActualMfrName"),
        description=p.get("Description"),
        category=p.get("Category"),
        package=package,
        url=p.get("ProductDetailUrl"),
        datasheet_url=p.get("DataSheetUrl") or None,
        image_urls=images,
        price_breaks=_breaks(p.get("PriceBreaks"), currency),
        stock=Stock(vendor="mouser", code=code, quantity=qty, in_stock=(qty or 0) > 0,
                    delivery=(f"factory lead time {p['LeadTime']}" if p.get("LeadTime") else None),
                    detail={"availability_text": p.get("Availability"), "on_order": on_order,
                            "factory_stock": to_int(re.sub(r"[^\d]", "", str(p.get("FactoryStock") or "")) or None)}),
        attributes=attrs,
        extra={
            "min_order_qty": to_int(p.get("Min")),
            "order_multiple": to_int(p.get("Mult")),
            "lead_time": p.get("LeadTime"),
            "lifecycle": p.get("LifecycleStatus"),
            "discontinued": str(p.get("IsDiscontinued") or "").lower() == "true",
            "rohs": bool(rohs) and "compliant" in rohs.casefold() and "non" not in rohs.casefold(),
            "rohs_status": rohs or None,
            "reach_svhc": [x for x in p.get("REACH-SVHC") or []],
            "weight_kg": to_float((p.get("UnitWeightKg") or {}).get("UnitWeight")),
            "reeling": bool(p.get("Reeling")),
            "alternate_packagings": [a.get("APMfrPN") for a in p.get("AlternatePackagings") or []],
            "suggested_replacement": p.get("SuggestedReplacement") or None,
            "compliance": {c.get("ComplianceName"): c.get("ComplianceValue") for c in (p.get("ProductCompliance") or []) + (p.get("TradeCompliance") or [])},
            "max_order_qty": to_int(p.get("SalesMaximumOrderQty")),
            "mouser_category": p.get("MouserProductCategory"),
            "info_messages": p.get("InfoMessages") or [],
        },
        raw=p,
    )


class Mouser:
    def __init__(self, http: Optional[Http] = None, api_key: Optional[str] = None, *, currency: str = "USD") -> None:
        self.http = http or Http(min_interval=2.0, cache_ttl=300.0)
        self.api_key = api_key or os.environ.get("MOUSER_API_KEY")
        self.currency = currency  # only used if a price break reports no currency
        self._easyeda = _easyeda.EasyEDA(Http())

    def is_configured(self) -> bool:
        return bool(self.api_key)

    # -- transport -----------------------------------------------------------------------
    def _post(self, path: str, root: str, body: dict[str, Any]) -> dict[str, Any]:
        if not self.api_key:
            raise AuthError("Mouser needs MOUSER_API_KEY (get a Search API key at https://www.mouser.com/api-hub/)")
        data = self.http.post_json(f"{BASE}{path}?apiKey={self.api_key}", {root: body})
        errors = data.get("Errors") or []
        if errors:
            msg = "; ".join(f"{e.get('Code')}: {e.get('Message')}" for e in errors)
            if any((e.get("PropertyName") or "").lower() == "api key" for e in errors):
                raise AuthError(f"Mouser rejected the API key ({msg})")
            if re.search(r"too ?many|limit|throttl", msg, re.I):
                raise BlockedError(f"Mouser rate limit: {msg}", 429)
            raise PartsApiError(f"Mouser error: {msg}")
        return data.get("SearchResults") or {}

    def _keyword(self, kw: str, start: int, records: int, options: str) -> dict[str, Any]:
        return self._post("/v1/search/keyword", "SearchByKeywordRequest",
                          {"keyword": kw[:250], "records": records, "startingRecord": start, "searchOptions": options})

    # -- search ---------------------------------------------------------------------------
    def findpart(
        self,
        query: str,
        *,
        limit: int = 25,
        where: Union[None, str, PartFilter] = None,
        sort: Union[None, str, Callable[[Part], Any]] = None,
        reverse: bool = False,
        qty: int = 1,
        max_pages: int = 3,
        parse: bool = True,
        strict: Optional[bool] = None,
    ) -> list[Part]:
        """Search Mouser with the full filter syntax of :mod:`partsapi.filters`.

        ``is:instock`` and ``is:rohs`` go to Mouser itself; all other filters run client-side over up to
        ``max_pages`` pages of 50 (one API call each). With ``sort`` every page is read first, so the order is
        over those candidates only. A search word is required.
        """
        if not 1 <= limit <= 1000:
            raise ValueError("limit must be 1..1000")
        pq = parse_query(query, qty=qty, strict=strict) if parse else ParsedQuery([query] if query.strip() else [], [], PartFilter(qty=qty))
        flt = pq.filter
        if where is not None:
            flt.merge(build_filter(where, qty))
        reject_unsupported(flt, "mouser", flags={"preferred", "basic", "extended"}, has={"3dmodel"}, library=True)
        queries = [k for k in pq.server_queries() if k.strip()]
        if not queries:
            raise ValueError("Mouser search needs at least one search word")
        stock, rohs = "instock" in flt.flags, "rohs" in flt.flags
        options = "RohsAndInStock" if stock and rohs else "InStock" if stock else "Rohs" if rohs else "None"
        records = PAGE if (not flt.empty or sort) else min(limit, PAGE)
        wanted = limit if not sort else 10**9
        found: dict[str, Part] = {}
        for kw in queries:
            for pg in range(max_pages):
                res = self._keyword(kw, pg * records, records, options)
                rows = res.get("Parts") or []
                for raw in rows:
                    part = _parse(raw, self.currency)
                    if part.code and part.code not in found and flt.matches(part):
                        found[part.code] = part
                if len(found) >= wanted or len(rows) < records or (pg + 1) * records >= (to_int(res.get("NumberOfResult")) or 0):
                    break
            if len(found) >= wanted:
                break
        out = list(found.values())
        if sort:
            out = sort_parts(out, sort, reverse=reverse, qty=qty)
        return out[:limit]

    def facets(self, query: str, **kw: Any) -> dict[str, Any]:
        """Counts over the search results (Mouser has no server-side facets)."""
        return _facets(self.findpart(query, **kw))

    def manufacturers(self) -> list[str]:
        """Mouser's manufacturer names (V2 ``manufacturerlist``)."""
        if not self.api_key:
            raise AuthError("Mouser needs MOUSER_API_KEY")
        data = self.http.get_json(f"{BASE}/v2/search/manufacturerlist?apiKey={self.api_key}")
        return [m["ManufacturerName"] for m in ((data.get("MouserManufacturerList") or {}).get("ManufacturerList") or [])]

    # -- single part ------------------------------------------------------------------------
    def getpart(self, code: str) -> Part:
        """Everything Mouser has on a Mouser part number (``595-LM358DR``) or an exact manufacturer part number."""
        code = code.strip()
        if not code or "|" in code:
            raise ValueError("pass exactly one part number")
        want = _squash(code)
        for opt in ("Exact", "None"):
            res = self._post("/v1/search/partnumber", "SearchByPartRequest", {"mouserPartNumber": code, "partSearchOptions": opt})
            for raw in res.get("Parts") or []:
                if want in (_squash(raw.get("MouserPartNumber") or ""), _squash(raw.get("ManufacturerPartNumber") or "")):
                    return _parse(raw, self.currency)
        for raw in (self._keyword(code, 0, 10, "None").get("Parts") or []):
            if want in (_squash(raw.get("MouserPartNumber") or ""), _squash(raw.get("ManufacturerPartNumber") or "")):
                return _parse(raw, self.currency)
        raise NotFoundError(f"Mouser has no part {code!r}")

    def getpartprice(self, code: str, qty: Optional[int] = None) -> Union[list[PriceBreak], float]:
        breaks = self.getpart(code).price_breaks
        if qty is None:
            return breaks
        price = pick_price(breaks, qty)
        if price is None:
            raise NotFoundError(f"{code} has no price on Mouser")
        return price

    getprice = getpartprice

    def getstock(self, code: str) -> Stock:
        s = self.getpart(code).stock
        assert s is not None
        return s

    def getattributes(self, code: str) -> dict[str, str]:
        return self.getpart(code).attributes

    def getpics(self, code: str) -> list[str]:
        return self.getpart(code).image_urls

    def getpic(self, code: str, save_to: Optional[PathLike] = None, index: int = 0) -> Union[bytes, Path]:
        part = self.getpart(code)
        if not part.image_urls:
            raise NotFoundError(f"{code} has no picture on Mouser")
        url = part.image_urls[min(index, len(part.image_urls) - 1)]
        data = self.http.get_bytes(url)
        return save_bytes(data, save_to, f"{safe_name(part.code)}{ext_from_url(url, '.jpg')}") if save_to else data

    def getdatasheet(self, code: str, save_to: Optional[PathLike] = None) -> Union[bytes, Path]:
        part = self.getpart(code)
        if not part.datasheet_url:
            raise NotFoundError(f"{code} has no datasheet on Mouser")
        data = self.http.get_bytes(part.datasheet_url, headers={"Referer": "https://www.mouser.com/"})
        return save_bytes(data, save_to, f"{safe_name(part.code)}.pdf") if save_to else data

    def get3dmodel(self, code: str, fmt: str = "step", save_to: Optional[PathLike] = None) -> Model3D:
        """Mouser hosts no 3D models through the API; the manufacturer part number is matched exactly against the
        LCSC/EasyEDA library (``Model3D.source_code`` names the LCSC part used)."""
        mpn = self.getpart(code).mpn
        if not mpn:
            raise PartsApiError(f"{code}: Mouser lists no manufacturer part number")
        return self._easyeda.get3dmodel_by_mpn(mpn, fmt, save_to)


_default = Mouser()


def is_configured() -> bool: return _default.is_configured()
def findpart(query: str, **kw: Any) -> list[Part]: return _default.findpart(query, **kw)
def facets(query: str, **kw: Any) -> dict[str, Any]: return _default.facets(query, **kw)
def manufacturers() -> list[str]: return _default.manufacturers()
def getpart(code: str) -> Part: return _default.getpart(code)
def getpartprice(code: str, qty: Optional[int] = None): return _default.getpartprice(code, qty)
getprice = getpartprice
def getstock(code: str) -> Stock: return _default.getstock(code)
def getattributes(code: str) -> dict[str, str]: return _default.getattributes(code)
def getpics(code: str) -> list[str]: return _default.getpics(code)
def getpic(code: str, save_to: Optional[PathLike] = None, index: int = 0): return _default.getpic(code, save_to, index)
def getdatasheet(code: str, save_to: Optional[PathLike] = None): return _default.getdatasheet(code, save_to)
def get3dmodel(code: str, fmt: str = "step", save_to: Optional[PathLike] = None) -> Model3D: return _default.get3dmodel(code, fmt, save_to)
