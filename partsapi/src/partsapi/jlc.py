"""JLCPCB parts library (https://jlcpcb.com/parts) - search, price, stock, pictures, datasheets, 3D.

Uses the same JSON endpoints the JLCPCB site calls. They are undocumented and may change.

Typical use::

    from partsapi import jlc
    jlc.findpart("10k 0603 resistor", library="basic", limit=5)
    jlc.getpartprice("C25804", qty=100)
    jlc.getstock("C25804")
    jlc.getpic("C25804", save_to="out/")
    jlc.get3dmodel("C25804", fmt="step", save_to="out/")
    jlc.getpart("C25804")        # everything, as a Part
"""
from __future__ import annotations

from pathlib import Path
from typing import Any, Mapping, Optional, Sequence, Union

from . import easyeda as _easyeda
from ._http import Http
from ._util import PathLike, ext_from_url, is_lcsc_code, safe_name, save_bytes, to_int
from .errors import NotFoundError, PartsApiError
from .models import Model3D, Part, PriceBreak, Stock, pick_price

BASE = "https://jlcpcb.com"
SEARCH = "/api/overseas-pcb-order/v1/shoppingCart/smtGood/selectSmtComponentList/v2"
MAX_PAGE_SIZE = 72
_LIB = {"base": "basic", "expand": "extended"}
_LIB_REQ = {"basic": "base", "base": "base", "extended": "expand", "expand": "expand"}


def _breaks(items: Optional[list[dict[str, Any]]]) -> list[PriceBreak]:
    out = []
    for p in items or []:
        end = p.get("endNumber")
        out.append(
            PriceBreak(
                min_qty=int(p["startNumber"]),
                max_qty=None if end in (None, -1) else int(end),
                unit_price=float(p["productPrice"]),
                currency="USD",
            )
        )
    return sorted(out, key=lambda b: b.min_qty)


def _parse(item: dict[str, Any]) -> Part:
    code = item["componentCode"]
    attrs = {a["attribute_name_en"]: a["attribute_value_name"] for a in item.get("attributes") or [] if a.get("attribute_name_en")}
    images = [u for u in (item.get("productBigImageAccessIdUrl"), item.get("minImageAccessIdUrl")) if u]
    datasheet = item.get("dataManualUrl") or item.get("dataManualFileAccessIdUrl") or item.get("dataManualOfficialLink") or None
    lib = item.get("componentLibraryType")
    return Part(
        vendor="jlc",
        code=code,
        mpn=item.get("componentModelEn"),
        manufacturer=item.get("componentBrandEn"),
        description=item.get("describe"),
        category=" / ".join(c for c in (item.get("secondSortName"), item.get("componentTypeEn")) if c) or None,
        package=item.get("componentSpecificationEn"),
        url=item.get("lcscGoodsUrl"),
        datasheet_url=datasheet,
        image_urls=images,
        price_breaks=_breaks(item.get("componentPrices")),
        stock=Stock(
            vendor="jlc",
            code=code,
            quantity=to_int(item.get("stockCount")),
            in_stock=(to_int(item.get("stockCount")) or 0) > 0,
            delivery=None,
            detail={"presale_available": to_int(item.get("canPresaleNumber")), "lead_days": item.get("estimateDate")},
        ),
        attributes=attrs,
        extra={
            "library_type": _LIB.get(lib, lib),  # "basic" | "extended"
            "preferred": bool(item.get("preferredComponentFlag")),
            "assembly_ok": bool(item.get("assemblyComponentFlag", True)),
            "rohs": bool(item.get("rohsFlag")),
            "min_order_qty": item.get("minPurchaseNum"),
            "reel_qty": item.get("encapsulationNumber"),
            "buy_price_breaks": [b.__dict__ for b in _breaks(item.get("buyComponentPrices"))],
            "jlc_component_id": item.get("componentId"),
            "image_urls_expire": "JLCPCB image/datasheet OSS links are signed and expire after ~30 min",
        },
        raw=item,
    )


class JLC:
    def __init__(self, http: Optional[Http] = None) -> None:
        self.http = http or Http(headers={"Origin": BASE, "Referer": f"{BASE}/parts"})
        self._easyeda = _easyeda.EasyEDA(self.http)

    # -- search / lookup ---------------------------------------------------------------
    def findpart(
        self,
        query: str,
        *,
        limit: int = 25,
        page: int = 1,
        library: Optional[str] = None,  # "basic" | "extended"
        preferred: Optional[bool] = None,
        in_stock_only: bool = False,
        min_stock: Optional[int] = None,
        brands: Optional[Sequence[str]] = None,
        packages: Optional[Sequence[str]] = None,
        attributes: Optional[Mapping[str, Sequence[str]]] = None,
        category: Optional[str] = None,
        extra: Optional[Mapping[str, Any]] = None,
    ) -> list[Part]:
        """Keyword search (MPN, LCSC code, description). Returns up to ``limit`` parts (max 72 per page)."""
        if not 1 <= limit <= MAX_PAGE_SIZE:
            raise ValueError(f"limit must be 1..{MAX_PAGE_SIZE}; page through results with page=")
        body: dict[str, Any] = {
            "currentPage": page,
            "pageSize": limit,
            "searchType": 2,
            "searchSource": "search",
            "keyword": query,
            "presaleType": "stock" if in_stock_only else "",
            "componentBrandList": list(brands or []),
            "componentSpecificationList": list(packages or []),
            "componentAttributeList": [{k: ([v] if isinstance(v, str) else list(v))} for k, v in (attributes or {}).items()],
        }
        if library:
            if library.lower() not in _LIB_REQ:
                raise ValueError("library must be 'basic' or 'extended'")
            body["componentLibraryType"] = _LIB_REQ[library.lower()]
        if preferred is not None:
            body["preferredComponentFlag"] = preferred
        if min_stock is not None:
            body["startStockNumber"] = min_stock
        if category:
            body["firstSortName"] = category
        if extra:
            body.update(extra)
        data = self.http.post_json(BASE + SEARCH, body)
        if data.get("code") != 200:
            raise PartsApiError(f"JLCPCB search failed: {data.get('message') or data}")
        return [_parse(i) for i in data["data"]["componentPageInfo"].get("list") or []]

    def getpart(self, code: str) -> Part:
        """Everything JLCPCB knows about one LCSC/JLCPCB part number (e.g. ``C25804``)."""
        code = code.strip().upper()
        if not is_lcsc_code(code):
            raise ValueError(f"{code!r} is not a JLCPCB/LCSC part number (expected e.g. 'C25804')")
        for part in self.findpart(code, limit=10):
            if part.code.upper() == code:
                return part
        raise NotFoundError(f"JLCPCB has no part {code}")

    # -- focused getters ---------------------------------------------------------------
    def getpartprice(self, code: str, qty: Optional[int] = None) -> Union[list[PriceBreak], float]:
        """Price ladder (USD) for ``code``; with ``qty`` return just the unit price at that quantity."""
        breaks = self.getpart(code).price_breaks
        if qty is None:
            return breaks
        price = pick_price(breaks, qty)
        if price is None:
            raise NotFoundError(f"{code} has no price on JLCPCB")
        return price

    getprice = getpartprice

    def getstock(self, code: str) -> Stock:
        part = self.getpart(code)
        assert part.stock is not None
        return part.stock

    def getdatasheet(self, code: str, save_to: Optional[PathLike] = None) -> Union[bytes, Path]:
        """Download the datasheet PDF (bytes, or the saved path when ``save_to`` is given)."""
        part = self.getpart(code)
        urls = [u for u in (part.raw.get("dataManualFileAccessIdUrl"), part.raw.get("dataManualUrl"), part.raw.get("dataManualOfficialLink")) if u]
        if not urls:
            raise NotFoundError(f"{code} has no datasheet on JLCPCB")
        data = self._first_download(urls)
        return save_bytes(data, save_to, f"{safe_name(part.code)}.pdf") if save_to else data

    def getpics(self, code: str) -> list[str]:
        """Image URLs (large first). NOTE: JLCPCB's links are signed and expire after ~30 minutes."""
        return self.getpart(code).image_urls

    def getpic(self, code: str, save_to: Optional[PathLike] = None, big: bool = True) -> Union[bytes, Path]:
        part = self.getpart(code)
        key = "productBigImageAccessIdUrl" if big else "minImageAccessIdUrl"
        urls = [part.raw.get(key), part.raw.get("minImageAccessIdUrl"), part.raw.get("productBigImageAccessIdUrl")]
        data_url = next((u for u in urls if u), None)
        if not data_url:
            raise NotFoundError(f"{code} has no picture on JLCPCB")
        data = self.http.get_bytes(data_url)
        return save_bytes(data, save_to, f"{safe_name(part.code)}{ext_from_url(data_url, '.jpg')}") if save_to else data

    def get3dmodel(self, code: str, fmt: str = "step", save_to: Optional[PathLike] = None) -> Model3D:
        """3D model (``fmt`` = 'step' or 'obj') from the EasyEDA library. Raises ``NoModelError`` if absent."""
        return self._easyeda.get3dmodel(code, fmt, save_to)

    def getattributes(self, code: str) -> dict[str, str]:
        return self.getpart(code).attributes

    def _first_download(self, urls: list[str]) -> bytes:
        last: Optional[Exception] = None
        for u in urls:
            try:
                return self.http.get_bytes(u)
            except PartsApiError as exc:
                last = exc
        raise NotFoundError(f"download failed: {last}")


_default = JLC()


def findpart(query: str, **kw: Any) -> list[Part]: return _default.findpart(query, **kw)
def getpart(code: str) -> Part: return _default.getpart(code)
def getpartprice(code: str, qty: Optional[int] = None): return _default.getpartprice(code, qty)
getprice = getpartprice
def getstock(code: str) -> Stock: return _default.getstock(code)
def getdatasheet(code: str, save_to: Optional[PathLike] = None): return _default.getdatasheet(code, save_to)
def getpics(code: str) -> list[str]: return _default.getpics(code)
def getpic(code: str, save_to: Optional[PathLike] = None, big: bool = True): return _default.getpic(code, save_to, big)
def get3dmodel(code: str, fmt: str = "step", save_to: Optional[PathLike] = None) -> Model3D: return _default.get3dmodel(code, fmt, save_to)
def getattributes(code: str) -> dict[str, str]: return _default.getattributes(code)
