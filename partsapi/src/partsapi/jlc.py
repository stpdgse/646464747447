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

import re
from pathlib import Path
from typing import Any, Callable, Mapping, Optional, Sequence, Union

from . import easyeda as _easyeda
from ._http import Http
from ._util import PathLike, ext_from_url, is_lcsc_code, safe_name, save_bytes, to_int
from .errors import NotFoundError, PartsApiError
from .filters import ParsedQuery, PartFilter, build_filter, parse_query, parse_quantity, sort_parts
from .models import Model3D, Part, PriceBreak, Stock, pick_price

BASE = "https://jlcpcb.com"
SEARCH = "/api/overseas-pcb-order/v1/shoppingCart/smtGood/selectSmtComponentList/v2"
FILTER = "/api/overseas-pcb-order/v1/componentSearch/filterComponentAttribute"
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
        query: str = "",
        *,
        limit: int = 25,
        page: int = 1,
        where: Union[None, str, PartFilter] = None,
        sort: Union[None, str, Callable[[Part], Any]] = None,
        reverse: bool = False,
        qty: int = 1,
        max_pages: int = 5,
        parse: bool = True,
        strict: Optional[bool] = None,
        library: Optional[str] = None,  # "basic" | "extended"
        preferred: Optional[bool] = None,
        in_stock_only: bool = False,
        min_stock: Optional[int] = None,
        brands: Optional[Sequence[str]] = None,
        packages: Optional[Sequence[str]] = None,
        attributes: Optional[Mapping[str, Sequence[str]]] = None,
        category: Optional[str] = None,
        subcategory: Optional[str] = None,
        has_datasheet: Optional[bool] = None,
        has_image: Optional[bool] = None,
        assembly_ok: Optional[bool] = None,
        extra: Optional[Mapping[str, Any]] = None,
    ) -> list[Part]:
        """Search the JLCPCB parts library.

        ``query`` accepts the full filter syntax (see :mod:`partsapi.filters`): ``"10k 0603 -thick lib:basic
        res:>=10k mfr:uni|yageo is:instock price:<0.01"``. Pass ``parse=False`` to send it verbatim instead.
        ``strict``: must bare words also appear in the part text? ``None`` = only words with digits (see
        :func:`partsapi.filters.parse_query`). ``where`` adds more conditions (string or :class:`PartFilter`). Whatever JLCPCB can filter itself
        (library, preferred, stock, exact package/brand, datasheet/image) is pushed to the server; the rest
        is checked client-side while paging through up to ``max_pages`` pages of 72. With ``sort`` the pages
        are all read first, so "cheapest" means cheapest of those pages, not of the whole catalogue.

        Exact server-side filters: ``brands`` / ``packages`` (exact names, see :meth:`facets`),
        ``attributes={"Tolerance": ["±1%"]}``, ``category="Resistors"``, ``subcategory="Chip Resistor - Surface Mount"``.
        """
        if not 1 <= limit <= 5000:
            raise ValueError("limit must be 1..5000")
        if library and library.lower() not in _LIB_REQ:
            raise ValueError("library must be 'basic' or 'extended'")
        pq = parse_query(query, qty=qty, strict=strict) if parse else ParsedQuery([query] if query else [], [], PartFilter(qty=qty))
        flt = pq.filter
        if where is not None:
            flt.merge(build_filter(where, qty))
        if library:
            flt.flags.add(_LIB_REQ[library.lower()] == "base" and "basic" or "extended")
        if preferred:
            flt.flags.add("preferred")
        if in_stock_only:
            flt.flags.add("instock")
        if min_stock is not None:
            flt.add_field("stock", f">={min_stock}")
        if has_datasheet:
            flt.has.add("datasheet")
        if has_image:
            flt.has.add("image")

        body: dict[str, Any] = {
            "searchType": 2, "searchSource": "search", "presaleType": "",
            "componentBrandList": list(brands or []),
            "componentSpecificationList": list(packages or []),
            "componentAttributeList": [{k: ([v] if isinstance(v, str) else list(v))} for k, v in (attributes or {}).items()],
        }
        body.update(self._pushdown(flt))
        scope = self._scope(category, subcategory, flt)
        if scope:
            body["firstSortName"], body["secondSortName"] = scope[0] or body.get("firstSortName"), scope[1] or body.get("secondSortName")
            body = {k: v for k, v in body.items() if v is not None}
            resolved = self._resolve_attrs(flt, scope)
            if resolved is not None:
                if not resolved:
                    return []  # an attribute condition no catalogue value can satisfy
                body["componentAttributeList"] = list(body["componentAttributeList"]) + resolved
        if preferred is False:
            body["preferredComponentFlag"] = False
        if assembly_ok is not None:
            body["assemblyComponentFlag"] = assembly_ok
        if extra:
            body.update(extra)

        client_side = not flt.empty
        wanted = limit if not sort else 10**9
        page_size = MAX_PAGE_SIZE if (client_side or sort) else min(limit, MAX_PAGE_SIZE)
        found: dict[str, Part] = {}
        for kw in pq.server_queries():
            for pg in range(page, page + max_pages):
                data = self.http.post_json(BASE + SEARCH, {**body, "keyword": kw or None, "currentPage": pg, "pageSize": page_size})
                if data.get("code") != 200:
                    raise PartsApiError(f"JLCPCB search failed: {data.get('message') or data}")
                info = data["data"]["componentPageInfo"]
                rows = info.get("list") or []
                for part in map(_parse, rows):
                    if part.code not in found and flt.matches(part):
                        found[part.code] = part
                if len(found) >= wanted or len(rows) < page_size or pg * page_size >= (info.get("total") or 0):
                    break
            if len(found) >= wanted:
                break
        out = list(found.values())
        if sort:
            out = sort_parts(out, sort, reverse=reverse, qty=qty)
        return out[:limit]

    @staticmethod
    def _pushdown(flt: PartFilter) -> dict[str, Any]:
        """The part of ``flt`` JLCPCB can evaluate itself (anything not listed stays client-side)."""
        b: dict[str, Any] = {}
        if "basic" in flt.flags:
            b["componentLibraryType"] = "base"
        elif "extended" in flt.flags:
            b["componentLibraryType"] = "expand"
        if "preferred" in flt.flags:
            b["preferredComponentFlag"] = True
        if "instock" in flt.flags or "stock" in flt.has:
            b["presaleType"] = "stock"
        if "datasheet" in flt.has:
            b["dateSheet"] = True
        if "image" in flt.has or "photo" in flt.has or "picture" in flt.has:
            b["photo"] = True
        for name, cond in flt.fields:
            if cond.negate:
                continue
            if name == "stock" and cond.op in ("ge", "gt") and (q := parse_quantity(cond.operands[0])):
                b["startStockNumber"] = int(q.value) + (1 if cond.op == "gt" else 0)
            elif name == "library" and cond.op in ("eq", "auto") and len(cond.operands) == 1 and cond.operands[0].lower() in _LIB_REQ:
                b["componentLibraryType"] = _LIB_REQ[cond.operands[0].lower()]
            elif name == "package" and cond.op == "eq":
                b.setdefault("componentSpecificationList", []).extend(cond.operands)
            elif name == "manufacturer" and cond.op == "eq":
                b.setdefault("componentBrandList", []).extend(cond.operands)
        return b

    # -- category scope and server-side attribute resolution ----------------------------
    def _tree(self) -> list[dict[str, Any]]:
        """Category tree with ids: ``[{id, name, children: [{id, name}]}]`` (cached by the HTTP layer)."""
        body = {"baseQueryDto": {"componentBrandList": [], "componentSpecificationList": [], "componentTypeIdList": [],
                                 "productTypeIdList": [], "orderLibraryTypeList": [], "packageTypeList": [], "filterType": 1,
                                 "keyword": None, "queryShelveStatus": None}, "catalogLevel": 1, "nowCondition": "",
                "paramList": [], "queryString": ""}
        d = self.http.post_json(BASE + FILTER, body)["data"]
        return [{"id": c["key"], "name": c["name"], "children": [{"id": k["key"], "name": k["name"]} for k in c.get("subAggs") or []]}
                for c in d.get("productTypeAggs") or []]

    @staticmethod
    def _cat_key(s: str) -> str:
        k = re.sub(r"[^a-z0-9]+", " ", s.casefold()).strip()
        return k[:-1] if k.endswith("s") else k

    def _find_category(self, name: str) -> Optional[tuple[str, Optional[str]]]:
        """Resolve a category / subcategory name to ``(top_name, sub_name_or_None)``; ``None`` if unknown or ambiguous."""
        want = self._cat_key(name)
        tree = self._tree()
        tops = [c for c in tree if self._cat_key(c["name"]) == want]
        if len(tops) == 1:
            return tops[0]["name"], None
        subs = [(c["name"], k["name"]) for c in tree for k in c["children"] if self._cat_key(k["name"]) == want]
        return subs[0] if len(subs) == 1 else None

    def _scope(self, category: Optional[str], subcategory: Optional[str], flt: PartFilter) -> Optional[tuple[Optional[str], Optional[str]]]:
        """The (top, sub) category this search is confined to, from arguments or a ``cat:`` condition."""
        if category or subcategory:
            top, sub = category, subcategory
            if category and not subcategory:
                r = self._find_category(category)
                if r:
                    top, sub = r
            return top, sub
        for name, cond in flt.fields:
            if name == "category" and not cond.negate and cond.op in ("auto", "eq") and len(cond.operands) == 1:
                r = self._find_category(cond.operands[0])
                if r:
                    return r
        return None

    def _resolve_attrs(self, flt: PartFilter, scope: tuple[Optional[str], Optional[str]]) -> Optional[list[dict[str, list[str]]]]:
        """Turn attribute conditions (``res:>=10k``) into exact value lists JLCPCB can filter on.

        Looks up the real values in the category (facets), keeps those passing the condition, and returns
        ``componentAttributeList`` entries. Conditions whose name matches several attributes, or that are
        negated, stay client-side. ``None`` = nothing to push.
        """
        todo = [(n, c, e) for n, c, e in flt.attrs if not c.negate]
        if not todo:
            return None
        values = self.facets("", category=scope[0], subcategory=scope[1])["attributes"]
        out: list[dict[str, list[str]]] = []
        for name, cond, exact in todo:
            names = [k for k in values if flt.attr_name_matches(k, name, exact)]
            if len(names) != 1:
                continue
            ok = [v for v, _ in values[names[0]] if cond.test(v)]
            if not ok:
                return []
            out.append({names[0]: ok})
        return out or None

    def facets(self, query: str = "", *, brands: Sequence[str] = (), packages: Sequence[str] = (),
               library: Optional[str] = None, category: Optional[str] = None,
               subcategory: Optional[str] = None) -> dict[str, Any]:
        """What filters are available for ``query``, with counts (server-side, whole catalogue).

        Returns ``total``, ``categories`` (nested), ``brands``, ``packages``, ``attributes``
        (``{"Resistance": [("10kΩ", 1234), ...]}``) and ``counts`` (basic / preferred / with datasheet / photo).
        Use the exact names it returns for ``brands=`` / ``packages=`` / ``attributes=``.
        """
        pq = parse_query(query)
        top_ids: list[str] = []
        sub_ids: list[str] = []
        if category or subcategory:
            tree = self._tree()
            top_name, sub_name = (category, subcategory)
            if category and not subcategory and (r := self._find_category(category)):
                top_name, sub_name = r
            top = [c for c in tree if self._cat_key(c["name"]) == self._cat_key(top_name or "")] if top_name else []
            if top_name and not top:
                raise NotFoundError(f"unknown JLCPCB category {top_name!r}; see facets()['categories']")
            top_ids = [c["id"] for c in top]
            if sub_name:
                subs = [k["id"] for c in (top or tree) for k in c["children"] if self._cat_key(k["name"]) == self._cat_key(sub_name)]
                if not subs:
                    raise NotFoundError(f"unknown JLCPCB subcategory {sub_name!r}")
                sub_ids = subs
        body = {
            "baseQueryDto": {
                "componentBrandList": list(brands), "componentSpecificationList": list(packages),
                "componentTypeIdList": sub_ids, "productTypeIdList": top_ids,
                "orderLibraryTypeList": [_LIB_REQ[library.lower()]] if library else [],
                "packageTypeList": [], "filterType": 1, "keyword": " ".join(pq.keywords) or None, "queryShelveStatus": None,
            },
            "catalogLevel": 1, "nowCondition": "", "paramList": [], "queryString": " ".join(pq.keywords),
        }
        data = self.http.post_json(BASE + FILTER, body)
        if data.get("code") != 200:
            raise PartsApiError(f"JLCPCB facets failed: {data.get('message') or data}")
        d = data["data"]
        agg = lambda xs: [(x["name"], x["docCount"]) for x in xs or [] if x.get("displayFlag", True)]  # noqa: E731
        cats = [{"name": c["name"], "count": c["docCount"],
                 "children": [{"name": k["name"], "count": k["docCount"]} for k in c.get("subAggs") or []]}
                for c in d.get("productTypeAggs") or d.get("productTypeList") or []]
        attrs = {a["name"]: [(v["name"], v["docCount"]) for v in a.get("subAggs") or []] for a in d.get("parentParamList") or []}
        return {
            "total": d.get("total"), "categories": cats, "brands": agg(d.get("componentBrandList")),
            "packages": agg(d.get("componentSpecificationList")), "attributes": attrs,
            "counts": {"basic": d.get("basePart"), "preferred": d.get("preferredPart"), "extended": d.get("extendPart"),
                       "with_datasheet": d.get("dataSheetPart"), "with_photo": d.get("photo")},
        }

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


def findpart(query: str = "", **kw: Any) -> list[Part]: return _default.findpart(query, **kw)
def facets(query: str = "", **kw: Any) -> dict[str, Any]: return _default.facets(query, **kw)
def getpart(code: str) -> Part: return _default.getpart(code)
def getpartprice(code: str, qty: Optional[int] = None): return _default.getpartprice(code, qty)
getprice = getpartprice
def getstock(code: str) -> Stock: return _default.getstock(code)
def getdatasheet(code: str, save_to: Optional[PathLike] = None): return _default.getdatasheet(code, save_to)
def getpics(code: str) -> list[str]: return _default.getpics(code)
def getpic(code: str, save_to: Optional[PathLike] = None, big: bool = True): return _default.getpic(code, save_to, big)
def get3dmodel(code: str, fmt: str = "step", save_to: Optional[PathLike] = None) -> Model3D: return _default.get3dmodel(code, fmt, save_to)
def getattributes(code: str) -> dict[str, str]: return _default.getattributes(code)
