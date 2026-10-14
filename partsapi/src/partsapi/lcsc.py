"""LCSC (https://www.lcsc.com) - search, price, stock, pictures, datasheets, symbol/footprint, 3D.

Part detail comes from LCSC's own JSON endpoint. LCSC's keyword-search endpoints sit behind bot
protection, so :func:`findpart` searches the identical LCSC catalogue through EasyEDA's component
search and then hydrates each hit from LCSC. A bare C-number skips the search entirely.

    from partsapi import lcsc
    lcsc.findpart("LM358", limit=5)
    lcsc.getpartprice("C7950", qty=100)
    lcsc.getstock("C7950")
    lcsc.getpic("C7950", save_to="out/")
    lcsc.get3dmodel("C2913204", save_to="out/")
    lcsc.set_currency("EUR")   # prices in EUR
"""
from __future__ import annotations

from pathlib import Path
from typing import Any, Callable, Optional, Union

from . import easyeda as _easyeda
from ._http import Http
from ._util import PathLike, ext_from_url, is_lcsc_code, safe_name, save_bytes, to_float, to_int
from .errors import NotFoundError, ParseError
from .filters import ParsedQuery, PartFilter, build_filter, facets as _facets, parse_query, reject_unsupported, sort_parts
from .models import Model3D, Part, PriceBreak, Stock, pick_price

DETAIL = "https://wmsc.lcsc.com/ftps/wm/product/detail"
SITE = "https://www.lcsc.com"


def _stub(hit: dict[str, Any]) -> Part:
    """A Part with only what EasyEDA's search result carries - enough for cheap pre-filtering."""
    cls = (hit.get("library_class") or "").lower()
    return Part(vendor="lcsc", code=hit["code"], mpn=hit.get("mpn"), manufacturer=hit.get("manufacturer"),
                package=hit.get("package"), description=hit.get("title"),
                extra={"library_type": "basic" if "basic" in cls else "extended" if "extended" in cls else None})


def _abs(url: Optional[str]) -> Optional[str]:
    if not url:
        return None
    return "https:" + url if url.startswith("//") else url


def _parse(r: dict[str, Any]) -> Part:
    code = r["productCode"]
    ladders = r.get("productPriceList") or []
    sym = r.get("currencySymbol") or "$"
    cur = r.get("currencyType") or "USD"
    breaks: list[PriceBreak] = []
    for i, p in enumerate(ladders):
        nxt = ladders[i + 1]["ladder"] - 1 if i + 1 < len(ladders) else None
        price = p.get("currencyPrice")
        if price is None:
            price = p.get("usdPrice") if p.get("usdPrice") is not None else p.get("productPrice")
        breaks.append(PriceBreak(min_qty=int(p["ladder"]), max_qty=nxt, unit_price=float(price), currency=cur))
    attrs = {p["paramNameEn"]: p.get("paramValueEn") or p.get("paramValue") for p in r.get("paramVOList") or [] if p.get("paramNameEn")}
    total = to_int(r.get("stockNumber"))
    overseas = r.get("overseasStockVO") or {}
    domestic = r.get("domesticStockVO") or {}
    return Part(
        vendor="lcsc",
        code=code,
        mpn=r.get("productModel"),
        manufacturer=r.get("brandNameEn"),
        description=r.get("productIntroEn") or r.get("productDescEn") or r.get("productNameEn"),
        category=" / ".join(c["catalogNameEn"] for c in r.get("parentCatalogList") or [] if c.get("catalogNameEn")) or r.get("catalogName"),
        package=r.get("encapStandard"),
        url=f"{SITE}/product-detail/{code}.html",
        datasheet_url=r.get("pdfUrl") or None,
        image_urls=[u for u in r.get("productImages") or [] if u],
        price_breaks=breaks,
        stock=Stock(
            vendor="lcsc",
            code=code,
            quantity=total,
            in_stock=(total or 0) > 0,
            delivery=None,
            detail={
                "overseas": overseas,
                "domestic": domestic,
                "ships_immediately": to_int(overseas.get("shipImmediately")),
                "ships_in_3_days": to_int(overseas.get("ship3Days")),
            },
        ),
        attributes=attrs,
        extra={
            "name": r.get("productNameEn"),
            "currency_symbol": sym,
            "min_order_qty": r.get("minBuyNumber"),
            "order_multiple": r.get("split"),
            "max_order_qty": None if r.get("maxBuyNumber") in (None, -1) else r.get("maxBuyNumber"),
            "package_qty": r.get("minPacketNumber"),
            "package_unit": r.get("minPacketUnit"),
            "reel_price": to_float(r.get("reelPrice")),
            "weight_g": to_float(r.get("weight")),
            "rohs": bool(r.get("isEnvironment")),
            "eccn": r.get("eccn"),
            "lead_time_days": r.get("productCycle"),
            "alternatives": [a.get("productCode") for a in r.get("alternatePartList") or [] if isinstance(a, dict)],
            "symbol_svg": _abs((r.get("edaSvgInfo") or {}).get("schSvg")),
            "footprint_svg": _abs((r.get("edaSvgInfo") or {}).get("pcbSvg")),
        },
        raw=r,
    )


class LCSC:
    def __init__(self, http: Optional[Http] = None, currency: str = "USD") -> None:
        self.http = http or Http(headers={"Origin": SITE, "Referer": SITE + "/"})
        self.currency = currency
        self._easyeda = _easyeda.EasyEDA(self.http)

    def getpart(self, code: str) -> Part:
        """Everything LCSC publishes for one C-number."""
        code = code.strip().upper()
        if not is_lcsc_code(code):
            raise ValueError(f"{code!r} is not an LCSC part number (expected e.g. 'C25804')")
        cookies = {"currencyCode": self.currency} if self.currency and self.currency != "USD" else None
        data = self.http.get_json(DETAIL, params={"productCode": code}, cookies=cookies)
        if data.get("code") != 200 or not data.get("result"):
            if data.get("code") in (404, 500) or not data.get("result"):
                raise NotFoundError(f"LCSC has no part {code} ({data.get('msg')})")
            raise ParseError(f"unexpected LCSC response: {str(data)[:200]}")
        return _parse(data["result"])

    def findpart(
        self,
        query: str,
        *,
        limit: int = 10,
        where: Union[None, str, PartFilter] = None,
        sort: Union[None, str, Callable[[Part], Any]] = None,
        reverse: bool = False,
        qty: int = 1,
        max_pages: int = 3,
        max_fetch: int = 60,
        parse: bool = True,
        strict: Optional[bool] = None,
    ) -> list[Part]:
        """Search by MPN / keyword / C-number, with the full filter syntax of :mod:`partsapi.filters`.

        The catalogue is searched through EasyEDA (up to ``max_pages`` pages of 100). Manufacturer, MPN,
        package and ``lib:basic|extended`` are checked on those cheap results first; only survivors are
        fetched in full from LCSC (at most ``max_fetch``), then price/stock/attribute/etc. filters apply.
        ``lib:`` is JLCPCB's class from the EasyEDA library; ``is:preferred`` is JLCPCB-only (use :mod:`jlc`).
        With ``sort`` every candidate is fetched first, so the order is over those candidates only.
        """
        q = query.strip()
        if is_lcsc_code(q) and not where:
            return [self.getpart(q)]
        pq = parse_query(q, qty=qty, strict=strict) if parse else ParsedQuery([q] if q else [], [], PartFilter(qty=qty))
        flt = pq.filter
        if where is not None:
            flt.merge(build_filter(where, qty))
        reject_unsupported(flt, "lcsc", flags={"preferred"}, has={"3dmodel"})
        queries = [k for k in pq.server_queries() if k.strip()]
        if not queries:
            raise ValueError("LCSC search needs at least one search word (filters alone are not enough; try jlc.findpart)")
        cheap_skip = {"text", "attrs", "price", "stock", "datasheet", "image", "rohs", "description", "category"}
        wanted = limit if not sort else 10**9
        found: dict[str, Part] = {}
        seen: set[str] = set()
        fetched = 0
        for kw in queries:
            for pg in range(1, max_pages + 1):
                hits = self._easyeda.search(kw, limit=100, page=pg)
                for h in hits:
                    code = h["code"]
                    if code in seen:
                        continue
                    seen.add(code)
                    if not flt.matches(_stub(h), cheap_skip):
                        continue
                    if fetched >= max_fetch:
                        break
                    fetched += 1
                    try:
                        part = self.getpart(code)
                    except NotFoundError:
                        continue  # in the EDA library but not (or no longer) sold
                    lib = _stub(h).extra.get("library_type")
                    if lib:
                        part.extra["library_type"] = lib
                    if flt.matches(part):
                        found[code] = part
                    if len(found) >= wanted:
                        break
                if len(found) >= wanted or len(hits) < 100 or fetched >= max_fetch:
                    break
            if len(found) >= wanted or fetched >= max_fetch:
                break
        out = list(found.values())
        if sort:
            out = sort_parts(out, sort, reverse=reverse, qty=qty)
        return out[:limit]

    def facets(self, query: str, **kw: Any) -> dict[str, Any]:
        """Counts of manufacturers, packages, attribute values, price and stock ranges over the search results
        (computed locally from :meth:`findpart`; LCSC has no server-side facets we can reach)."""
        return _facets(self.findpart(query, **kw))

    def getpartprice(self, code: str, qty: Optional[int] = None) -> Union[list[PriceBreak], float]:
        breaks = self.getpart(code).price_breaks
        if qty is None:
            return breaks
        price = pick_price(breaks, qty)
        if price is None:
            raise NotFoundError(f"{code} has no price on LCSC")
        return price

    getprice = getpartprice

    def getstock(self, code: str) -> Stock:
        s = self.getpart(code).stock
        assert s is not None
        return s

    def getattributes(self, code: str) -> dict[str, str]:
        return self.getpart(code).attributes

    def getdatasheet(self, code: str, save_to: Optional[PathLike] = None) -> Union[bytes, Path]:
        part = self.getpart(code)
        if not part.datasheet_url:
            raise NotFoundError(f"{code} has no datasheet on LCSC")
        data = self.http.get_bytes(part.datasheet_url, headers={"Referer": SITE + "/"})
        return save_bytes(data, save_to, f"{safe_name(part.code)}.pdf") if save_to else data

    def getpics(self, code: str) -> list[str]:
        """Stable 900x900 image URLs (front/back/...)."""
        return self.getpart(code).image_urls

    def getpic(self, code: str, save_to: Optional[PathLike] = None, index: int = 0) -> Union[bytes, Path]:
        part = self.getpart(code)
        if not part.image_urls:
            raise NotFoundError(f"{code} has no picture on LCSC")
        url = part.image_urls[min(index, len(part.image_urls) - 1)]
        data = self.http.get_bytes(url)
        return save_bytes(data, save_to, f"{safe_name(part.code)}{ext_from_url(url, '.jpg')}") if save_to else data

    def getsymbol(self, code: str, save_to: Optional[PathLike] = None) -> Union[bytes, Path]:
        """Schematic symbol as SVG."""
        return self._svg(code, "symbol_svg", "_symbol.svg", save_to)

    def getfootprint(self, code: str, save_to: Optional[PathLike] = None) -> Union[bytes, Path]:
        """PCB footprint as SVG."""
        return self._svg(code, "footprint_svg", "_footprint.svg", save_to)

    def _svg(self, code: str, key: str, suffix: str, save_to: Optional[PathLike]) -> Union[bytes, Path]:
        part = self.getpart(code)
        url = part.extra.get(key)
        if not url:
            raise NotFoundError(f"{code} has no {key.split('_')[0]} graphic on LCSC")
        data = self.http.get_bytes(url)
        return save_bytes(data, save_to, f"{safe_name(part.code)}{suffix}") if save_to else data

    def get3dmodel(self, code: str, fmt: str = "step", save_to: Optional[PathLike] = None) -> Model3D:
        return self._easyeda.get3dmodel(code, fmt, save_to)


_default = LCSC()


def set_currency(code: str) -> None: _default.currency = code
def findpart(query: str, **kw: Any) -> list[Part]: return _default.findpart(query, **kw)
def facets(query: str, **kw: Any) -> dict[str, Any]: return _default.facets(query, **kw)
def getpart(code: str) -> Part: return _default.getpart(code)
def getpartprice(code: str, qty: Optional[int] = None): return _default.getpartprice(code, qty)
getprice = getpartprice
def getstock(code: str) -> Stock: return _default.getstock(code)
def getattributes(code: str) -> dict[str, str]: return _default.getattributes(code)
def getdatasheet(code: str, save_to: Optional[PathLike] = None): return _default.getdatasheet(code, save_to)
def getpics(code: str) -> list[str]: return _default.getpics(code)
def getpic(code: str, save_to: Optional[PathLike] = None, index: int = 0): return _default.getpic(code, save_to, index)
def getsymbol(code: str, save_to: Optional[PathLike] = None): return _default.getsymbol(code, save_to)
def getfootprint(code: str, save_to: Optional[PathLike] = None): return _default.getfootprint(code, save_to)
def get3dmodel(code: str, fmt: str = "step", save_to: Optional[PathLike] = None) -> Model3D: return _default.get3dmodel(code, fmt, save_to)
