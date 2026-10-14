"""Adafruit (https://www.adafruit.com) via its public JSON product API.

No key needed. Adafruit publishes the catalogue as JSON (``/api/products``, ``/api/product/<id>``) and documents the
product API in its own Learn guides. Its ``robots.txt`` asks crawlers to stay away from ``/api/`` and ``/search``, so this
module is deliberately *not* a crawler: it downloads the catalogue once (about 8 MB, cached on disk for 24 hours, see
``PARTSAPI_CACHE_DIR``), searches it locally, and fetches single products only on demand. It never calls the site search.
Requests identify themselves as ``partsapi``.

    from partsapi import adafruit
    adafruit.findpart("feather rp2040 is:instock", sort="price")
    adafruit.getpartprice("4884", qty=10)
    adafruit.getpart("732")            # + description, linked documents (datasheet PDF) from the product page
    adafruit.getdatasheet("2652")

Adafruit sells its own boards plus resold parts; it has no manufacturer part numbers (``Part.mpn`` is Adafruit's own
``ADA1234``), no parametric attributes in the catalogue and no 3D models through the API. Stock: "in stock" without a
count, or a number; zero or negative means out of stock.
"""
from __future__ import annotations

import ast
import re
import time
from pathlib import Path
from typing import Any, Callable, Optional, Union

from bs4 import BeautifulSoup

from ._cache import DiskCache, default_dir
from ._http import Http
from ._util import PathLike, ext_from_url, parse_price, safe_name, save_bytes, to_float, to_int
from .errors import NoModelError, NotFoundError
from .filters import PartFilter, build_filter, facets as _facets, parse_query, reject_unsupported, sort_parts
from .models import Model3D, Part, PriceBreak, Stock, pick_price

BASE = "https://www.adafruit.com"
USER_AGENT = "partsapi/0.1 (+https://github.com/stpdgse/646464747447)"
CATALOG_MAX_AGE = 24 * 3600.0
CATEGORY_MAX_AGE = 30 * 24 * 3600.0
_UNSET: Any = object()


def _none(v: Any) -> Optional[str]:
    s = None if v is None else str(v).strip()
    return None if s in (None, "", "None", "null", "0") else s


def _stock(raw: Any, discontinued: bool) -> tuple[Optional[int], bool]:
    """Adafruit's ``product_stock`` is "in stock" or a number; <= 0 means none (negative = backordered/discontinued)."""
    s = str(raw or "").strip().lower()
    if s == "in stock":
        return None, True
    n = to_int(s)
    if n is None:
        return None, False
    return (n, True) if n > 0 else (0, False)


def _tiers(raw: Any) -> list[PriceBreak]:
    """``discount_pricing`` is a list of dicts, or (in older data) the Python repr of one."""
    if isinstance(raw, str):
        try:
            raw = ast.literal_eval(raw)
        except (ValueError, SyntaxError):
            return []
    rows = []
    for t in raw or []:
        price, qty = parse_price(t.get("discounted_price")), to_int(t.get("min_qty"))
        if price is not None and qty:
            rows.append((qty, price))
    rows.sort()
    return [PriceBreak(q, (rows[i + 1][0] - 1 if i + 1 < len(rows) else None), p, "USD") for i, (q, p) in enumerate(rows)]


def _id_of(code: str) -> str:
    """'4884', 'ADA4884', 'https://www.adafruit.com/product/4884' -> '4884'."""
    m = re.fullmatch(r"\s*(?:https?://[^\s/]+/product/|ADA)?(\d+)(?:[/?#]\S*)?\s*", code, re.I)
    if not m:
        raise ValueError(f"{code!r} is not an Adafruit product id (expected e.g. '4884')")
    return m.group(1)


def _parse(p: dict[str, Any], category: Optional[str] = None) -> Part:
    pid = str(p["product_id"])
    discontinued = str(p.get("discontinue_status") or "").lower() == "discontinued"
    qty, in_stock = _stock(p.get("product_stock"), discontinued)
    price = parse_price(p.get("product_price"))
    breaks = _tiers(p.get("discount_pricing")) or ([PriceBreak(1, None, price, "USD")] if price is not None else [])
    image = p.get("product_image") or None
    images = [image.replace("/640x480/", "/1200x900/"), image] if image and "/640x480/" in image else ([image] if image else [])
    desc_html = p.get("products_description") or ""
    return Part(
        vendor="adafruit",
        code=pid,
        mpn=_none(p.get("product_mpn")),
        manufacturer=_none(p.get("product_manufacturer")),
        description=p.get("product_name"),
        category=category,
        url=p.get("product_url") or f"{BASE}/product/{pid}",
        image_urls=images,
        price_breaks=breaks,
        stock=Stock(vendor="adafruit", code=pid, quantity=qty, in_stock=in_stock and not (discontinued and not qty),
                    detail={"raw": p.get("product_stock"), "coming_soon": p.get("products_coming_soon") == "1"}),
        extra={
            "model": _none(p.get("product_model")),
            "category_id": _none(p.get("product_master_category")),
            "discontinued": discontinued,
            "discontinue_status": _none(p.get("discontinue_status")),
            "rohs": str(p.get("products_rohs")) == "1",
            "hts": _none(p.get("products_hts")),
            "country_of_origin": _none(p.get("products_coo")),
            "shipping_weight": to_float(p.get("product_shipping_weight")),
            "date_added": p.get("date_added"),
            "parent_id": _none(p.get("parent_pid")),
            "sale_price": parse_price(p.get("product_sale_price")),
            "description_html": desc_html or None,
        },
        raw=p,
    )


class Adafruit:
    def __init__(self, http: Optional[Http] = None, *, cache_dir: Any = _UNSET, max_age: float = CATALOG_MAX_AGE) -> None:
        self.http = http or Http(min_interval=1.0, headers={"User-Agent": USER_AGENT})
        self.cache = DiskCache(default_dir() if cache_dir is _UNSET else (Path(cache_dir) if cache_dir else None))
        self.max_age = max_age
        self._catalog: Optional[list[dict[str, Any]]] = None
        self._loaded = 0.0
        self._cats: Optional[dict[str, str]] = None  # id -> "Parent / Child" path
        self._names: dict[str, str] = {}

    # -- catalogue / categories ------------------------------------------------------------
    def catalog(self, refresh: bool = False) -> list[dict[str, Any]]:
        """All products (``/api/products``): memory -> disk (24 h) -> one download."""
        if self._catalog is not None and not refresh and time.monotonic() - self._loaded < self.max_age:
            return self._catalog
        data = None if refresh else self.cache.get("adafruit_products.json", self.max_age)
        if data is None:
            data = self.http.request("GET", f"{BASE}/api/products", timeout=120).json()
            if not isinstance(data, list):
                raise NotFoundError("Adafruit returned an unexpected catalogue")
            self.cache.put("adafruit_products.json", data)
        self._catalog, self._loaded = data, time.monotonic()
        return data

    def categories(self) -> dict[str, str]:
        """``{category_id: "Parent / Child"}`` for all ~670 categories. Downloads Adafruit's category tree (about 28 MB,
        cached for 30 days) - only done when you filter with ``cat:`` or ask for the tree."""
        if self._cats is None:
            data = self.cache.get("adafruit_category_paths.json", CATEGORY_MAX_AGE)
            if data is None:
                tree = self.http.request("GET", f"{BASE}/api/categories", timeout=300).json()
                data = {}

                def walk(nodes: list[dict[str, Any]], parents: tuple[str, ...]) -> None:
                    for n in nodes:
                        path = parents + (n["category_name"],)
                        data[str(n["category_id"])] = " / ".join(path)
                        walk(n.get("subcategories") or [], path)

                walk(tree, ())
                self.cache.put("adafruit_category_paths.json", data)
            self._cats = data
        return self._cats

    def _category_name(self, cid: Optional[str]) -> Optional[str]:
        """Cheap per-id lookup (one tiny request, remembered) used for result lists."""
        if not cid:
            return None
        if self._cats and cid in self._cats:
            return self._cats[cid]
        if cid not in self._names:
            cached = self.cache.get("adafruit_category_names.json", CATEGORY_MAX_AGE) or {}
            self._names.update(cached)
            if cid not in self._names:
                try:
                    self._names[cid] = self.http.get_json(f"{BASE}/api/category/{cid}").get("category_name") or ""
                except Exception:  # noqa: BLE001 - a missing category name must never break a search
                    self._names[cid] = ""
                self.cache.put("adafruit_category_names.json", self._names)
        return self._names[cid] or None

    # -- search ------------------------------------------------------------------------------
    def findpart(
        self,
        query: str,
        *,
        limit: int = 25,
        where: Union[None, str, PartFilter] = None,
        sort: Union[None, str, Callable[[Part], Any]] = None,
        reverse: bool = False,
        qty: int = 1,
        parse: bool = True,
        strict: Optional[bool] = None,
        include_discontinued: bool = False,
    ) -> list[Part]:
        """Search Adafruit's catalogue locally with the full filter syntax of :mod:`partsapi.filters`.

        Every search word must appear in the product name / number (case, spaces and dashes ignored); results are ranked
        by relevance (exact number, name starts with the query) then in-stock first unless you pass ``sort``.
        Discontinued products are skipped unless ``include_discontinued=True``. ``cat:`` loads the category tree once.
        Not available: ``mfr:`` is mostly empty, no attributes / datasheets / 3D in the catalogue.
        """
        if limit < 1:
            raise ValueError("limit must be >= 1")
        pq = parse_query(query, qty=qty, strict=True if strict is None else strict) if parse else parse_query("", qty=qty)
        flt = pq.filter
        if not parse and query.strip():
            flt.all_terms.append(query.strip())
        if where is not None:
            flt.merge(build_filter(where, qty))
        reject_unsupported(flt, "adafruit", flags={"preferred", "basic", "extended"}, has={"3dmodel", "datasheet"}, library=True)
        need_cats = any(n == "category" for n, _ in flt.fields)
        cats = self.categories() if need_cats else {}
        words = [w.casefold() for w in (pq.keywords if parse else [query]) if w]
        found: list[tuple[tuple[int, bool, str], Part]] = []
        for raw in self.catalog():
            if not include_discontinued and str(raw.get("discontinue_status") or "").lower() == "discontinued":
                continue
            part = _parse(raw, cats.get(str(raw.get("product_master_category"))))
            if not flt.matches(part):
                continue
            name = (part.description or "").casefold()
            rank = 0 if (part.code == _digits(query) or (part.mpn or "").casefold() in words) else 1 if words and name.startswith(words[0]) else 2
            found.append(((rank, not (part.stock and part.stock.in_stock), name), part))
        found.sort(key=lambda t: t[0])
        out = [p for _, p in found]
        if sort:
            out = sort_parts(out, sort, reverse=reverse, qty=qty)
        out = out[:limit]
        if not cats:
            for p in out:
                p.category = self._category_name(p.extra.get("category_id"))
        return out

    def facets(self, query: str, **kw: Any) -> dict[str, Any]:
        """Counts over the search results (computed locally)."""
        return _facets(self.findpart(query, **kw))

    # -- single product -------------------------------------------------------------------
    def getpart(self, code: str) -> Part:
        """One product by id (``4884``, ``ADA4884`` or its URL), with description text and the PDFs linked on its page."""
        pid = _id_of(code)
        raw = self.http.get_json(f"{BASE}/api/product/{pid}")
        if not isinstance(raw, dict) or not raw.get("product_id"):
            raise NotFoundError(f"Adafruit has no product {pid}")
        raw = dict(raw)
        raw["products_description"] = raw.pop("products_description", None) or raw.get("product_description") or ""
        part = _parse(raw, self._category_name(_none(raw.get("product_master_category"))))
        soup = BeautifulSoup(raw["products_description"], "lxml")
        part.extra["summary"] = soup.get_text(" ", strip=True)[:600] or None
        for li in soup.find_all("li"):  # "Key: value" bullets, when a product has them
            k, sep, v = li.get_text(" ", strip=True).partition(":")
            if sep and 0 < len(k) <= 40 and v.strip():
                part.attributes.setdefault(k.strip(), v.strip())
        # The API description has no download links; the product page lists them ("Downloads: Datasheet for ...").
        try:
            html = self.http.get_text(part.url or f"{BASE}/product/{pid}")
        except Exception:  # noqa: BLE001 - documents are a bonus; never fail getpart for them
            html = ""
        docs = []
        for m in re.finditer(r'<a[^>]+href="([^"]+?\.pdf)(?:\?[^"]*)?"[^>]*>(.*?)</a>', html, re.I | re.S):
            url = m.group(1) if m.group(1).startswith("http") else BASE + m.group(1)
            if url not in [d["url"] for d in docs]:
                docs.append({"url": url, "title": BeautifulSoup(m.group(2), "lxml").get_text(" ", strip=True) or None})
        learn = re.search(r'href="(https://learn\.adafruit\.com/[^"]*/downloads)"', html)
        part.extra["documents"] = docs
        part.extra["learn_downloads_url"] = learn.group(1) if learn else None
        part.datasheet_url = next((d["url"] for d in docs if "datasheet" in (d["url"] + (d["title"] or "")).lower()), docs[0]["url"] if docs else None)
        return part

    def getpartprice(self, code: str, qty: Optional[int] = None) -> Union[list[PriceBreak], float]:
        breaks = self.getpart(code).price_breaks
        if qty is None:
            return breaks
        price = pick_price(breaks, qty)
        if price is None:
            raise NotFoundError(f"{code} has no price on Adafruit")
        return price

    getprice = getpartprice

    def getstock(self, code: str) -> Stock:
        s = self.getpart(code).stock
        assert s is not None
        return s

    def getattributes(self, code: str) -> dict[str, str]:
        """``Key: value`` bullets from the description, where a product has them (often none; Adafruit has no parametric data)."""
        return self.getpart(code).attributes

    def getpics(self, code: str) -> list[str]:
        return self.getpart(code).image_urls

    def getpic(self, code: str, save_to: Optional[PathLike] = None, index: int = 0) -> Union[bytes, Path]:
        part = self.getpart(code)
        if not part.image_urls:
            raise NotFoundError(f"{code} has no picture on Adafruit")
        data, url = None, ""
        for url in part.image_urls[index:] or part.image_urls:  # biggest first, fall back to the 640x480
            try:
                data = self.http.get_bytes(url)
                break
            except Exception:  # noqa: BLE001
                continue
        if data is None:
            raise NotFoundError(f"could not download a picture for {code}")
        return save_bytes(data, save_to, f"adafruit_{safe_name(part.code)}{ext_from_url(url, '.jpg')}") if save_to else data

    def getdatasheet(self, code: str, save_to: Optional[PathLike] = None) -> Union[bytes, Path]:
        """First PDF linked under the product page's downloads (preferring one called "datasheet")."""
        part = self.getpart(code)
        if not part.datasheet_url:
            hint = f"; its tutorial has a downloads page: {part.extra['learn_downloads_url']}" if part.extra.get("learn_downloads_url") else ""
            raise NotFoundError(f"{code}: no PDF linked on its Adafruit product page{hint}")
        data = self.http.get_bytes(part.datasheet_url)
        return save_bytes(data, save_to, f"adafruit_{safe_name(part.code)}.pdf") if save_to else data

    def get3dmodel(self, code: str, fmt: str = "step", save_to: Optional[PathLike] = None) -> Model3D:
        raise NoModelError("Adafruit does not offer 3D models through its product API")


def _digits(s: str) -> str:
    m = re.fullmatch(r"\s*(?:ADA)?(\d+)\s*", s or "", re.I)
    return m.group(1) if m else ""


_default = Adafruit()


def findpart(query: str, **kw: Any) -> list[Part]: return _default.findpart(query, **kw)
def facets(query: str, **kw: Any) -> dict[str, Any]: return _default.facets(query, **kw)
def catalog(refresh: bool = False) -> list[dict[str, Any]]: return _default.catalog(refresh)
def categories() -> dict[str, str]: return _default.categories()
def getpart(code: str) -> Part: return _default.getpart(code)
def getpartprice(code: str, qty: Optional[int] = None): return _default.getpartprice(code, qty)
getprice = getpartprice
def getstock(code: str) -> Stock: return _default.getstock(code)
def getattributes(code: str) -> dict[str, str]: return _default.getattributes(code)
def getpics(code: str) -> list[str]: return _default.getpics(code)
def getpic(code: str, save_to: Optional[PathLike] = None, index: int = 0): return _default.getpic(code, save_to, index)
def getdatasheet(code: str, save_to: Optional[PathLike] = None): return _default.getdatasheet(code, save_to)
def get3dmodel(code: str, fmt: str = "step", save_to: Optional[PathLike] = None) -> Model3D: return _default.get3dmodel(code, fmt, save_to)
