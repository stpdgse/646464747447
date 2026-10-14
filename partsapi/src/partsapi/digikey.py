"""DigiKey (https://www.digikey.com) via the official Product Information API v4.

Needs a (free) API app from https://developer.digikey.com/ with the *Product Information V4* product
enabled. Credentials come from the environment or the constructor::

    export DIGIKEY_CLIENT_ID=...   DIGIKEY_CLIENT_SECRET=...      (DK_CLIENT_ID / DK_CLIENT_SECRET also work)
    # optional: DIGIKEY_SITE=US  DIGIKEY_LANGUAGE=en  DIGIKEY_CURRENCY=USD  DIGIKEY_SANDBOX=1

Two-legged OAuth2 (client credentials) is used, so no browser login is needed.

    from partsapi import digikey
    digikey.findpart("cat:resistor res:10k..100k tol:<=1% pkg:0603 is:instock", sort="price", limit=10)
    digikey.getpartprice("296-1395-1-ND", qty=100)
    digikey.getstock("LM358DR")
    digikey.getpic("LM358DR", save_to="out/")
    digikey.get3dmodel("LM358DR")          # via the manufacturer part number, see README

Written against DigiKey's published v4 OpenAPI schema. DigiKey rate-limits per app; a filtered ``findpart`` may use
several calls (one per page of 50), so ``max_pages`` bounds it.
"""
from __future__ import annotations

import os
import re
import time
from pathlib import Path
from typing import Any, Callable, Optional, Union
from urllib.parse import quote

from . import easyeda as _easyeda
from ._http import Http
from ._util import PathLike, ext_from_url, safe_name, save_bytes, to_int
from .errors import AuthError, BlockedError, NotFoundError, PartsApiError
from .filters import (ParsedQuery, PartFilter, build_filter, parse_query, parse_quantity,
                      reject_unsupported, sort_parts)
from .models import Model3D, Part, PriceBreak, Stock, pick_price

PROD = "https://api.digikey.com"
SANDBOX = "https://sandbox-api.digikey.com"
PAGE = 50  # API maximum per request
SORT_FIELDS = {"price": "Price", "stock": "QuantityAvailable", "mpn": "ManufacturerProductNumber",
               "code": "DigiKeyProductNumber", "manufacturer": "Manufacturer"}
PKG_PARAMS = ("package / case", "supplier device package", "package")


def _env(name: str) -> Optional[str]:
    """``DIGIKEY_<NAME>``, falling back to the short ``DK_<NAME>`` spelling."""
    return os.environ.get(f"DIGIKEY_{name}") or os.environ.get(f"DK_{name}") or None


def _abs(url: Optional[str]) -> Optional[str]:
    if not url:
        return None
    return "https:" + url if url.startswith("//") else url


def _category_path(node: Optional[dict[str, Any]]) -> Optional[str]:
    names: list[str] = []
    while node:
        if node.get("Name"):
            names.append(node["Name"])
        kids = node.get("ChildCategories") or node.get("Children") or []
        node = kids[0] if kids else None
    return " / ".join(names) or None


def _breaks(items: Optional[list[dict[str, Any]]], currency: str) -> list[PriceBreak]:
    rows = sorted((i for i in items or [] if i.get("BreakQuantity") is not None), key=lambda i: i["BreakQuantity"])
    return [PriceBreak(min_qty=int(r["BreakQuantity"]), max_qty=(int(rows[i + 1]["BreakQuantity"]) - 1 if i + 1 < len(rows) else None),
                       unit_price=float(r["UnitPrice"]), currency=currency) for i, r in enumerate(rows)]


def _cat_key(s: str) -> str:
    k = re.sub(r"[^a-z0-9]+", " ", s.casefold()).strip()
    return k[:-1] if k.endswith("s") else k


def _parse(prod: dict[str, Any], currency: str = "USD", code: Optional[str] = None) -> Part:
    """Map a v4 ``Product`` to a :class:`Part`. ``code`` picks the variation (package type) to price."""
    variations = prod.get("ProductVariations") or []
    chosen = next((v for v in variations if code and (v.get("DigiKeyProductNumber") or "").casefold() == code.casefold()), None) \
        or (variations[0] if variations else {})
    params = prod.get("Parameters") or []
    attrs = {p["ParameterText"]: p.get("ValueText") for p in params if p.get("ParameterText")}
    package = next((attrs[k] for k in attrs if k.casefold() in PKG_PARAMS), None)
    cls = prod.get("Classifications") or {}
    rohs_text = cls.get("RohsStatus") or ""
    status = (prod.get("ProductStatus") or {}).get("Status")
    mfr = (prod.get("Manufacturer") or {}).get("Name")
    qty = to_int(prod.get("QuantityAvailable"))
    mpn = prod.get("ManufacturerProductNumber")
    var_info = [{"digikey_number": v.get("DigiKeyProductNumber"), "package_type": (v.get("PackageType") or {}).get("Name"),
                 "quantity": to_int(v.get("QuantityAvailableforPackageType")), "min_order_qty": to_int(v.get("MinimumOrderQuantity")),
                 "standard_package": to_int(v.get("StandardPackage")), "marketplace": bool(v.get("MarketPlace")),
                 "price_breaks": [b.__dict__ for b in _breaks(v.get("StandardPricing"), currency)]} for v in variations]
    desc = prod.get("Description") or {}
    return Part(
        vendor="digikey",
        code=chosen.get("DigiKeyProductNumber") or mpn or "",
        mpn=mpn,
        manufacturer=mfr,
        description=desc.get("DetailedDescription") or desc.get("ProductDescription"),
        category=_category_path(prod.get("Category")),
        package=package,
        url=prod.get("ProductUrl"),
        datasheet_url=_abs(prod.get("DatasheetUrl")),
        image_urls=[u for u in (_abs(prod.get("PhotoUrl")),) if u],
        price_breaks=_breaks(chosen.get("StandardPricing"), currency),
        stock=Stock(vendor="digikey", code=chosen.get("DigiKeyProductNumber") or mpn or "", quantity=qty,
                    in_stock=(qty or 0) > 0, delivery=(f"manufacturer lead time {prod['ManufacturerLeadWeeks']} weeks"
                                                       if prod.get("ManufacturerLeadWeeks") else None),
                    detail={"variations": var_info, "manufacturer_stock": to_int(prod.get("ManufacturerPublicQuantity")),
                            "lead_weeks": prod.get("ManufacturerLeadWeeks")}),
        attributes=attrs,
        extra={
            "name": desc.get("ProductDescription"),
            "series": (prod.get("Series") or {}).get("Name"),
            "status": status,
            "discontinued": bool(prod.get("Discontinued")),
            "end_of_life": bool(prod.get("EndOfLife")),
            "normally_stocking": bool(prod.get("NormallyStocking")),
            "rohs": bool(rohs_text) and "compliant" in rohs_text.casefold() and "non" not in rohs_text.casefold(),
            "rohs_status": rohs_text or None,
            "reach_status": cls.get("ReachStatus"),
            "moisture_sensitivity": cls.get("MoistureSensitivityLevel"),
            "eccn": cls.get("ExportControlClassNumber"),
            "htsus": cls.get("HtsusCode"),
            "min_order_qty": to_int(chosen.get("MinimumOrderQuantity")),
            "standard_package": to_int(chosen.get("StandardPackage")),
            "package_type": (chosen.get("PackageType") or {}).get("Name"),
            "variations": var_info,
            "other_names": prod.get("OtherNames") or [],
            "base_product": (prod.get("BaseProductNumber") or {}).get("Name"),
            "price_note": f"DigiKey standard pricing in {currency}; MyPricing is not used",
        },
        raw=prod,
    )


class DigiKey:
    def __init__(
        self,
        http: Optional[Http] = None,
        client_id: Optional[str] = None,
        client_secret: Optional[str] = None,
        *,
        site: Optional[str] = None,
        language: Optional[str] = None,
        currency: Optional[str] = None,
        sandbox: Optional[bool] = None,
    ) -> None:
        self.http = http or Http(min_interval=0.5)
        self.client_id = client_id or _env("CLIENT_ID")
        self.client_secret = client_secret or _env("CLIENT_SECRET")
        self.site = site or _env("SITE") or "US"
        self.language = language or _env("LANGUAGE") or "en"
        self.currency = currency or _env("CURRENCY") or "USD"
        self.sandbox = sandbox if sandbox is not None else (_env("SANDBOX") or "").lower() not in ("", "0", "false")
        self.base = SANDBOX if self.sandbox else PROD
        self._token: Optional[str] = None
        self._expires = 0.0
        self._mfrs: Optional[list[dict[str, Any]]] = None
        self._cats: Optional[list[dict[str, Any]]] = None
        self._easyeda = _easyeda.EasyEDA(self.http)

    def is_configured(self) -> bool:
        return bool(self.client_id and self.client_secret)

    # -- auth + transport --------------------------------------------------------------
    def _auth(self) -> str:
        if not self.is_configured():
            raise AuthError("DigiKey needs DIGIKEY_CLIENT_ID and DIGIKEY_CLIENT_SECRET (create an app at "
                            "https://developer.digikey.com/ and enable 'Product Information V4')")
        if self._token and time.monotonic() < self._expires:
            return self._token
        try:
            resp = self.http.request("POST", f"{self.base}/v1/oauth2/token", data={
                "client_id": self.client_id, "client_secret": self.client_secret, "grant_type": "client_credentials"})
        except BlockedError as exc:
            if exc.status in (401, 403):
                raise AuthError("DigiKey rejected the client id/secret") from exc
            raise
        data = self.http._json(resp, "DigiKey token endpoint")
        if "access_token" not in data:
            raise AuthError(f"DigiKey token response had no access_token: {str(data)[:200]}")
        self._token = data["access_token"]
        self._expires = time.monotonic() + max(30.0, float(data.get("expires_in", 600)) - 30.0)
        return self._token

    def _call(self, method: str, path: str, *, params: Optional[dict[str, Any]] = None, body: Optional[dict[str, Any]] = None) -> Any:
        for attempt in (0, 1):
            headers = {"Authorization": f"Bearer {self._auth()}", "X-DIGIKEY-Client-Id": self.client_id or "",
                       "X-DIGIKEY-Locale-Site": self.site, "X-DIGIKEY-Locale-Language": self.language,
                       "X-DIGIKEY-Locale-Currency": self.currency, "Accept": "application/json"}
            try:
                resp = self.http.request(method, f"{self.base}/products/v4{path}", params=params, json=body, headers=headers)
            except BlockedError as exc:
                if exc.status == 401 and attempt == 0:
                    self._token = None  # expired token: fetch a new one and retry once
                    continue
                if exc.status in (401, 403):
                    raise AuthError("DigiKey refused the request: token, client id or API subscription is invalid "
                                    "(is 'Product Information V4' enabled for your app?)") from exc
                raise
            return self.http._json(resp, path)
        raise AuthError("DigiKey authentication failed")  # pragma: no cover

    # -- reference lists -------------------------------------------------------------------
    def manufacturers(self) -> list[dict[str, Any]]:
        """All DigiKey manufacturers as ``[{Id, Name}]`` (cached per instance)."""
        if self._mfrs is None:
            self._mfrs = self._call("GET", "/search/manufacturers").get("Manufacturers") or []
        return self._mfrs

    def categories(self) -> list[dict[str, Any]]:
        """The DigiKey category tree (cached per instance)."""
        if self._cats is None:
            self._cats = self._call("GET", "/search/categories").get("Categories") or []
        return self._cats

    def _category_ids(self, cond_operands: list[str], exact: bool) -> list[int]:
        flat: list[tuple[int, str]] = []

        def walk(nodes: list[dict[str, Any]]) -> None:
            for n in nodes:
                flat.append((int(n["CategoryId"]), n.get("Name") or ""))
                walk(n.get("Children") or n.get("ChildCategories") or [])

        walk(self.categories())
        ids: list[int] = []
        for op in cond_operands:
            want = _cat_key(op)
            ids += [i for i, name in flat if (_cat_key(name) == want if exact else want and want in _cat_key(name))]
        return list(dict.fromkeys(ids))

    def _manufacturer_ids(self, cond_operands: list[str], exact: bool) -> list[int]:
        ids: list[int] = []
        for op in cond_operands:
            w = op.casefold()
            ids += [int(m["Id"]) for m in self.manufacturers()
                    if (m["Name"].casefold() == w if exact else w in m["Name"].casefold())]
        return list(dict.fromkeys(ids))

    # -- search ----------------------------------------------------------------------------
    def _search(self, kw: str, offset: int, limit: int, fo: dict[str, Any], sort: Optional[tuple[str, bool]]) -> dict[str, Any]:
        body: dict[str, Any] = {"Keywords": kw[:250], "Limit": limit, "Offset": offset}
        if fo:
            body["FilterOptionsRequest"] = fo
        if sort:
            body["SortOptions"] = {"Field": sort[0], "SortOrder": "Descending" if sort[1] else "Ascending"}
        return self._call("POST", "/search/keyword", body=body)

    def findpart(
        self,
        query: str,
        *,
        limit: int = 25,
        where: Union[None, str, PartFilter] = None,
        sort: Union[None, str, Callable[[Part], Any]] = None,
        reverse: bool = False,
        qty: int = 1,
        max_pages: int = 4,
        parse: bool = True,
        strict: Optional[bool] = None,
        extra: Optional[dict[str, Any]] = None,
    ) -> list[Part]:
        """Search DigiKey with the full filter syntax of :mod:`partsapi.filters`.

        Pushed to DigiKey: ``is:instock``, ``is:rohs``, ``has:datasheet|image|3dmodel``, ``stock:>=N``, manufacturer and
        category (by DigiKey id), numeric/text parametric filters (when the category's facet values provably cover all
        its products) and sorting by price / stock / mpn / code / manufacturer. Everything else is checked client-side
        while paging (50 per page, at most ``max_pages``). At least one search word is required; narrow with ``cat:``.
        ``extra`` is merged into ``FilterOptionsRequest`` (e.g. ``{"StatusFilter": [{"Id": "0"}]}``).
        """
        if not 1 <= limit <= 1000:
            raise ValueError("limit must be 1..1000")
        pq = parse_query(query, qty=qty, strict=strict) if parse else ParsedQuery([query] if query.strip() else [], [], PartFilter(qty=qty))
        flt = pq.filter
        if where is not None:
            flt.merge(build_filter(where, qty))
        reject_unsupported(flt, "digikey", flags={"preferred", "basic", "extended"}, library=True)
        queries = [k for k in pq.server_queries() if k.strip()]
        if not queries:
            raise ValueError("DigiKey search needs at least one search word (add one to the query; narrow it with cat:)")

        fo = self._filter_options(flt)
        if fo is None:
            return []  # a manufacturer/category name that DigiKey does not have
        if extra:
            fo.update(extra)
        server_sort = SORT_FIELDS.get(sort) if isinstance(sort, str) else None
        if sort == "price" and qty != 1:
            server_sort = None  # DigiKey sorts by single-unit price
        sort_arg = (server_sort, reverse) if server_sort else None
        wanted = limit if (not sort or server_sort) else 10**9
        currency = self.currency
        found: dict[str, Part] = {}
        for kw in queries:
            fo.pop("ParameterFilterRequest", None)      # resolved per search word; never leak into the next one
            pfr = self._resolve_params(kw, flt, fo)
            if pfr == "none":
                return []
            if pfr:
                fo["ParameterFilterRequest"] = pfr
            for pg in range(max_pages):
                page_size = PAGE if not flt.empty else min(PAGE, limit)
                data = self._search(kw, pg * page_size, page_size, fo, sort_arg)
                currency = (data.get("SearchLocaleUsed") or {}).get("Currency") or currency
                rows = (data.get("ExactMatches") or []) if pg == 0 else []
                rows = rows + (data.get("Products") or [])
                for prod in rows:
                    part = _parse(prod, currency)
                    if part.code and part.code not in found and flt.matches(part):
                        found[part.code] = part
                total = to_int(data.get("ProductsCount")) or 0
                if len(found) >= wanted or not data.get("Products") or (pg + 1) * page_size >= total:
                    break
            if len(found) >= wanted:
                break
        out = list(found.values())
        if sort:
            out = sort_parts(out, sort, reverse=reverse, qty=qty)
        return out[:limit]

    def _filter_options(self, flt: PartFilter) -> Optional[dict[str, Any]]:
        """The part of ``flt`` DigiKey evaluates itself; ``None`` if a name filter can never match anything."""
        fo: dict[str, Any] = {}
        opts: list[str] = []
        if "instock" in flt.flags or "stock" in flt.has:
            opts.append("InStock")
        if "rohs" in flt.flags:
            opts.append("RohsCompliant")
        for h, o in (("datasheet", "HasDatasheet"), ("image", "HasProductPhoto"), ("photo", "HasProductPhoto"),
                     ("picture", "HasProductPhoto"), ("3dmodel", "Has3DModel")):
            if h in flt.has and o not in opts:
                opts.append(o)
        if opts:
            fo["SearchOptions"] = opts
        mfr_done = cat_done = stock_done = False
        for name, cond in flt.fields:
            if cond.negate:
                continue
            if name == "stock" and not stock_done and cond.op in ("ge", "gt") and (q := parse_quantity(cond.operands[0])):
                fo["MinimumQuantityAvailable"] = int(q.value) + (1 if cond.op == "gt" else 0)
                stock_done = True
            elif name == "manufacturer" and not mfr_done and cond.op in ("auto", "eq"):
                ids = self._manufacturer_ids(cond.operands, exact=cond.op == "eq")
                if not ids:
                    return None
                if len(ids) <= 50:
                    fo["ManufacturerFilter"] = [{"Id": str(i)} for i in ids]
                    mfr_done = True
            elif name == "category" and not cat_done and cond.op in ("auto", "eq"):
                ids = self._category_ids(cond.operands, exact=cond.op == "eq") or self._category_ids(cond.operands, exact=True)
                if not ids:
                    return None
                if len(ids) <= 10:
                    fo["CategoryFilter"] = [{"Id": str(i)} for i in ids]
                    cat_done = True
        return fo

    def _resolve_params(self, kw: str, flt: PartFilter, fo: dict[str, Any]) -> Union[None, str, dict[str, Any]]:
        """Turn attribute conditions into DigiKey parametric filters, but only when it is provably complete.

        Needs exactly one category. DigiKey's facet values for the parameter must account for every product in
        the category (sum of counts == total); otherwise a value could be missing and results silently lost, so the
        condition stays client-side. Returns a ``ParameterFilterRequest``, ``"none"`` (no value can match) or ``None``.
        """
        todo = [(n, c, e) for n, c, e in flt.attrs if not c.negate]
        cats = fo.get("CategoryFilter") or []
        if not todo or len(cats) != 1:
            return None
        probe = self._search(kw, 0, 1, {k: v for k, v in fo.items() if k != "ParameterFilterRequest"}, None)
        total = to_int(probe.get("ProductsCount")) or 0
        facets = (probe.get("FilterOptions") or {}).get("ParametricFilters") or []
        out: list[dict[str, Any]] = []
        for name, cond, exact in todo:
            hits = [f for f in facets if flt.attr_name_matches(f.get("ParameterName") or "", name, exact)]
            if len(hits) != 1:
                continue
            vals = hits[0].get("FilterValues") or []
            if not total or sum(to_int(v.get("ProductCount")) or 0 for v in vals) < total:
                continue  # facet values do not provably cover the category
            ok = [v for v in vals if cond.test(v.get("ValueName"))]
            if not ok:
                return "none"
            out.append({"ParameterId": hits[0]["ParameterId"], "FilterValues": [{"Id": str(v["ValueId"])} for v in ok]})
        if not out:
            return None
        return {"CategoryFilter": {"Id": cats[0]["Id"]}, "ParameterFilters": out}

    def facets(self, query: str, *, top: int = 15, **kw: Any) -> dict[str, Any]:
        """What filters exist for ``query``. Server-side counts from DigiKey's ``FilterOptions`` (manufacturers,
        packaging, status, series, top categories, per-category parametric values), plus the total match count."""
        pq = parse_query(query, strict=kw.get("strict"))
        flt = pq.filter
        if kw.get("where") is not None:
            flt.merge(build_filter(kw["where"]))
        fo = self._filter_options(flt) or {}
        words = " ".join(pq.keywords)
        if not words:
            raise ValueError("DigiKey facets need at least one search word")
        data = self._search(words, 0, 1, fo, None)
        f = data.get("FilterOptions") or {}
        pair = lambda xs: [(x.get("Value"), x.get("ProductCount")) for x in xs or []][:top]  # noqa: E731
        return {
            "total": data.get("ProductsCount"),
            "manufacturers": pair(f.get("Manufacturers")), "packaging": pair(f.get("Packaging")),
            "status": pair(f.get("Status")), "series": pair(f.get("Series")),
            "categories": [((c.get("Category") or {}).get("Name"), (c.get("Category") or {}).get("ProductCount")) for c in f.get("TopCategories") or []][:top],
            "attributes": {p.get("ParameterName"): [(v.get("ValueName"), v.get("ProductCount")) for v in p.get("FilterValues") or []][:top]
                           for p in f.get("ParametricFilters") or []},
        }

    # -- single part -----------------------------------------------------------------------
    def getpart(self, code: str, *, manufacturer: Optional[str] = None) -> Part:
        """Everything DigiKey has on a DigiKey part number (``296-1395-1-ND``) or manufacturer part number.
        An MPN used by several manufacturers needs ``manufacturer="Texas Instruments"`` to disambiguate."""
        code = code.strip()
        if not code:
            raise ValueError("empty part number")
        params = None
        if manufacturer:
            ids = self._manufacturer_ids([manufacturer], exact=True) or self._manufacturer_ids([manufacturer], exact=False)
            if not ids:
                raise NotFoundError(f"DigiKey has no manufacturer {manufacturer!r}")
            params = {"manufacturerId": ids[0]}
        try:
            data = self._call("GET", f"/search/{quote(code, safe='')}/productdetails", params=params)
        except NotFoundError:
            data = {}
        prod = data.get("Product")
        if prod:
            return _parse(prod, (data.get("SearchLocaleUsed") or {}).get("Currency") or self.currency, code)
        # not a direct hit: look for an exact match in keyword results
        res = self._search(code, 0, 10, {}, None)
        for p in (res.get("ExactMatches") or []) + (res.get("Products") or []):
            part = _parse(p, (res.get("SearchLocaleUsed") or {}).get("Currency") or self.currency, code)
            numbers = {v["digikey_number"].casefold() for v in part.extra["variations"] if v["digikey_number"]}
            if (part.mpn or "").casefold() == code.casefold() or code.casefold() in numbers:
                return part
        raise NotFoundError(f"DigiKey has no part {code!r}")

    def getpartprice(self, code: str, qty: Optional[int] = None) -> Union[list[PriceBreak], float]:
        """Price ladder of the package type ``code`` names; with ``qty``, the unit price at that quantity.

        For a manufacturer part number (several package types) the cheapest package that can be ordered at ``qty``
        is used (its minimum order quantity must be <= ``qty``)."""
        part = self.getpart(code)
        if qty is None:
            return part.price_breaks
        is_variation = any((v["digikey_number"] or "").casefold() == code.strip().casefold() for v in part.extra["variations"])
        options = [part.price_breaks] if is_variation else [
            [PriceBreak(**b) for b in v["price_breaks"]] for v in part.extra["variations"]
            if v["price_breaks"] and (v["min_order_qty"] or 1) <= qty] or [part.price_breaks]
        prices = [p for p in (pick_price(b, qty) for b in options) if p is not None]
        if not prices:
            raise NotFoundError(f"{code} has no price on DigiKey")
        return min(prices)

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
            raise NotFoundError(f"{code} has no picture on DigiKey")
        url = part.image_urls[min(index, len(part.image_urls) - 1)]
        data = self.http.get_bytes(url)
        return save_bytes(data, save_to, f"{safe_name(part.code)}{ext_from_url(url, '.jpg')}") if save_to else data

    def getdatasheet(self, code: str, save_to: Optional[PathLike] = None) -> Union[bytes, Path]:
        part = self.getpart(code)
        if not part.datasheet_url:
            raise NotFoundError(f"{code} has no datasheet on DigiKey")
        data = self.http.get_bytes(part.datasheet_url)
        return save_bytes(data, save_to, f"{safe_name(part.code)}.pdf") if save_to else data

    def get3dmodel(self, code: str, fmt: str = "step", save_to: Optional[PathLike] = None) -> Model3D:
        """DigiKey hosts no 3D models through the API; the manufacturer part number is matched exactly against the
        LCSC/EasyEDA library (``Model3D.source_code`` names the LCSC part used)."""
        mpn = self.getpart(code).mpn
        if not mpn:
            raise PartsApiError(f"{code}: DigiKey lists no manufacturer part number")
        return self._easyeda.get3dmodel_by_mpn(mpn, fmt, save_to)


_default = DigiKey()


def is_configured() -> bool: return _default.is_configured()
def findpart(query: str, **kw: Any) -> list[Part]: return _default.findpart(query, **kw)
def facets(query: str, **kw: Any) -> dict[str, Any]: return _default.facets(query, **kw)
def getpart(code: str, **kw: Any) -> Part: return _default.getpart(code, **kw)
def getpartprice(code: str, qty: Optional[int] = None): return _default.getpartprice(code, qty)
getprice = getpartprice
def getstock(code: str) -> Stock: return _default.getstock(code)
def getattributes(code: str) -> dict[str, str]: return _default.getattributes(code)
def getpics(code: str) -> list[str]: return _default.getpics(code)
def getpic(code: str, save_to: Optional[PathLike] = None, index: int = 0): return _default.getpic(code, save_to, index)
def getdatasheet(code: str, save_to: Optional[PathLike] = None): return _default.getdatasheet(code, save_to)
def get3dmodel(code: str, fmt: str = "step", save_to: Optional[PathLike] = None) -> Model3D: return _default.get3dmodel(code, fmt, save_to)
