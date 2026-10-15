"""SparkFun (https://www.sparkfun.com) via its public storefront GraphQL endpoint (Magento).

No key needed: ``https://www.sparkfun.com/graphql`` is the same endpoint the shop's own pages use and it is not
excluded in SparkFun's ``robots.txt`` (their HTML search pages are, and this module never touches them). Requests are
spaced 1 s apart and identify themselves as ``partsapi``.

    from partsapi import sparkfun
    sparkfun.findpart("qwiic sensor is:instock", sort="price", limit=10)
    sparkfun.findpart("sensor cat:environmental @\\"Sensor Type\\":humidity")     # server-side attribute filter
    sparkfun.getpartprice("SEN-19833", qty=10)
    sparkfun.getpart("SEN-19833")                                              # + datasheet / schematic links
    sparkfun.getdatasheet("SEN-19833", save_to="out/")

What SparkFun's API does not give: stock *counts* (only in / out of stock), per-product attributes (the GraphQL field
for them errors server-side), manufacturer part numbers (``Part.mpn`` is SparkFun's own SKU, e.g. ``SEN-19833``) and
3D models. Attribute *filters* still work: SparkFun's curated facets (e.g. "Form Factor", "Sensor Type") are resolved
to their option values and sent as server-side filters.
"""
from __future__ import annotations

import re
from pathlib import Path
from typing import Any, Callable, Optional, Union

from bs4 import BeautifulSoup

from ._http import Http
from ._util import PathLike, safe_name, save_bytes, to_int
from .errors import NoModelError, NotFoundError, PartsApiError
from .filters import ParsedQuery, PartFilter, build_filter, parse_query, reject_unsupported, sort_parts
from .models import Model3D, Part, PriceBreak, Stock, pick_price

BASE = "https://www.sparkfun.com"
GRAPHQL = f"{BASE}/graphql"
USER_AGENT = "partsapi/0.1 (+https://github.com/stpdgse/646464747447)"
PAGE = 100
SKIP_CATEGORIES = ("special-categories", "featured", "sale", "new-products")

_PRODUCT_FIELDS = """
  sku name uid url_key url_suffix stock_status only_x_left_in_stock min_sale_qty max_sale_qty country_of_manufacture
  short_description{html}
  image{url} media_gallery{url disabled}
  price_range{minimum_price{regular_price{value currency} final_price{value currency}}}
  price_tiers{quantity final_price{value currency}}
  categories{uid name url_path level}
"""


def _text(html: Optional[str], limit: int = 600) -> Optional[str]:
    if not html:
        return None
    return BeautifulSoup(html, "lxml").get_text(" ", strip=True)[:limit] or None


def _category_path(cats: list[dict[str, Any]]) -> Optional[str]:
    """Deepest real category as "Components / Buttons and Switches / Button" (skipping marketing categories)."""
    real = [c for c in cats if c.get("url_path") and not c["url_path"].split("/")[0] in SKIP_CATEGORIES]
    if not real:
        return None
    best = max(real, key=lambda c: (c.get("level") or 0))
    names = {c["url_path"]: c["name"] for c in real}
    parts = best["url_path"].split("/")
    chain = ["/".join(parts[: i + 1]) for i in range(len(parts))]
    return " / ".join(names[p] for p in chain if p in names) or best["name"]


def _parse(it: dict[str, Any]) -> Part:
    sku = it["sku"]
    base = ((it.get("price_range") or {}).get("minimum_price") or {})
    price = (base.get("final_price") or {})
    currency = price.get("currency") or "USD"
    rows = []
    if price.get("value") is not None:
        rows.append((1, float(price["value"])))
    rows += [(int(t["quantity"]), float(t["final_price"]["value"])) for t in it.get("price_tiers") or [] if t.get("quantity")]
    rows = sorted(dict(rows).items())
    breaks = [PriceBreak(q, (rows[i + 1][0] - 1 if i + 1 < len(rows) else None), p, currency) for i, (q, p) in enumerate(rows)]
    in_stock = it.get("stock_status") == "IN_STOCK"
    images: list[str] = []
    for u in [(it.get("image") or {}).get("url")] + [g.get("url") for g in it.get("media_gallery") or [] if not g.get("disabled")]:
        if u and u not in images:
            images.append(u)
    return Part(
        vendor="sparkfun",
        code=sku,
        mpn=sku,
        description=it.get("name"),
        category=_category_path(it.get("categories") or []),
        url=f"{BASE}/{it['url_key']}{it.get('url_suffix') or ''}" if it.get("url_key") else None,
        image_urls=images,
        price_breaks=breaks,
        stock=Stock(vendor="sparkfun", code=sku, quantity=None, in_stock=in_stock,
                    detail={"stock_status": it.get("stock_status"), "only_x_left_in_stock": it.get("only_x_left_in_stock")}),
        extra={
            "summary": _text((it.get("short_description") or {}).get("html")),
            "min_order_qty": to_int(it.get("min_sale_qty")),
            "max_order_qty": to_int(it.get("max_sale_qty")),
            "country_of_origin": it.get("country_of_manufacture"),
            "uid": it.get("uid"),
            "category_uids": [c["uid"] for c in it.get("categories") or [] if c.get("uid")],
        },
        raw=it,
    )


class SparkFun:
    def __init__(self, http: Optional[Http] = None) -> None:
        self.http = http or Http(min_interval=1.0, headers={"User-Agent": USER_AGENT, "Store": "default"})

    # -- transport -------------------------------------------------------------------------
    def _gql(self, query: str) -> dict[str, Any]:
        data = self.http.post_json(GRAPHQL, {"query": query}, headers={"Store": "default", "Content-Type": "application/json"})
        if data.get("errors") and not data.get("data"):
            raise PartsApiError(f"SparkFun GraphQL error: {data['errors'][0].get('message')}")
        return data.get("data") or {}

    @staticmethod
    def _q(s: str) -> str:
        return '"' + s.replace("\\", "\\\\").replace('"', '\\"') + '"'

    def _products(self, search: Optional[str], filt: dict[str, list[str]], page: int, size: int, with_aggs: bool = False) -> dict[str, Any]:
        args = [f"pageSize:{size}", f"currentPage:{page}"]
        if search:
            args.append(f"search:{self._q(search)}")
            args.append("sort:{relevance:DESC}")
        if filt:
            inner = ", ".join(f"{code}:{{in:[{', '.join(self._q(v) for v in values)}]}}" for code, values in filt.items())
            args.append(f"filter:{{{inner}}}")
        aggs = " aggregations{ label attribute_code count options{label value count} }" if with_aggs else ""
        body = f"{{ products({', '.join(args)}) {{ total_count page_info{{total_pages}} items{{{_PRODUCT_FIELDS}}}{aggs} }} }}"
        return self._gql(body).get("products") or {}

    def _category_uids(self, cond_operands: list[str], exact: bool) -> list[str]:
        uids: list[str] = []
        for op in cond_operands:
            data = self._gql(f'{{ categories(filters:{{name:{{match:{self._q(op)}}}}}){{ items{{ uid name url_path level }} }} }}')
            items = (data.get("categories") or {}).get("items") or []
            want = op.casefold()
            same = [i for i in items if i["name"].casefold() == want]
            uids += [i["uid"] for i in (same or ([] if exact else items)) if i.get("url_path", "").split("/")[0] not in SKIP_CATEGORIES]
        return list(dict.fromkeys(uids))

    # -- search ----------------------------------------------------------------------------
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
        """Search SparkFun with the full filter syntax of :mod:`partsapi.filters`.

        Sent to SparkFun: the search words (relevance order), ``cat:`` (resolved to category ids) and attribute filters
        whose name matches one of SparkFun's facets (``@"Form Factor":...``, ``@"Sensor Type":humidity``). Price, stock
        status and everything else is checked client-side over up to ``max_pages`` pages of 100. A search word is required.
        Not available: ``mfr:`` / ``is:rohs`` / ``has:datasheet`` / ``lib:`` (no such data in the API).
        """
        if limit < 1:
            raise ValueError("limit must be >= 1")
        pq = parse_query(query, qty=qty, strict=strict) if parse else ParsedQuery([query] if query.strip() else [], [], PartFilter(qty=qty))
        flt = pq.filter
        if where is not None:
            flt.merge(build_filter(where, qty))
        reject_unsupported(flt, "sparkfun", flags={"preferred", "basic", "extended", "rohs"}, has={"3dmodel", "datasheet"}, library=True)
        if any(n == "manufacturer" for n, _ in flt.fields):
            raise ValueError("sparkfun does not support mfr: (its API has no manufacturer data)")
        queries = [k for k in pq.server_queries() if k.strip()]
        if not queries:
            raise ValueError("SparkFun search needs at least one search word")
        filt: dict[str, list[str]] = {}
        for name, cond in flt.fields:
            if name == "category" and not cond.negate and cond.op in ("auto", "eq") and "category_uid" not in filt:
                uids = self._category_uids(cond.operands, exact=cond.op == "eq")
                if not uids:
                    return []
                filt["category_uid"] = uids
        wanted = limit if not sort else 10**9
        found: dict[str, Part] = {}
        for kw in queries:
            kfilt = dict(filt)
            if flt.attrs:
                resolved = self._resolve_attrs(kw, flt, kfilt)
                if resolved == "none":
                    continue
                kfilt.update(resolved or {})
            for pg in range(1, max_pages + 1):
                res = self._products(kw, kfilt, pg, PAGE)
                for it in res.get("items") or []:
                    part = _parse(it)
                    if part.code not in found and flt.matches(part, skip={"attrs"} if kfilt.keys() - filt.keys() else ()):
                        found[part.code] = part
                pages = ((res.get("page_info") or {}).get("total_pages")) or 0
                if len(found) >= wanted or pg >= pages:
                    break
            if len(found) >= wanted:
                break
        out = list(found.values())
        if sort:
            out = sort_parts(out, sort, reverse=reverse, qty=qty)
        return out[:limit]

    def _resolve_attrs(self, kw: str, flt: PartFilter, filt: dict[str, list[str]]) -> Union[None, str, dict[str, list[str]]]:
        """Turn attribute conditions into SparkFun facet filters (``code -> option values``).

        SparkFun's per-product attributes are not readable, so a condition that cannot be matched to exactly one facet
        would silently drop every product; such conditions raise instead of failing quietly."""
        probe = self._products(kw, filt, 1, 1, with_aggs=True)
        aggs = [a for a in probe.get("aggregations") or [] if a.get("attribute_code") not in ("price", "category_uid", "category_id")]
        out: dict[str, list[str]] = {}
        for name, cond, exact in flt.attrs:
            if cond.negate:
                raise ValueError(f"sparkfun cannot evaluate a negated attribute filter ({name!r}); its API has no per-product attributes")
            hits = [a for a in aggs if flt.attr_name_matches(a.get("label") or "", name, exact)]
            if len(hits) != 1:
                known = ", ".join(sorted(a["label"] for a in aggs)) or "none for this search"
                raise ValueError(f"no single SparkFun facet matches {name!r} for this search (available: {known})")
            ok = [o["value"] for o in hits[0].get("options") or [] if cond.test(o.get("label"))]
            if not ok:
                return "none"
            out[hits[0]["attribute_code"]] = ok
        return out or None

    def facets(self, query: str, **kw: Any) -> dict[str, Any]:
        """SparkFun's own facets for ``query`` (server-side): ``{label: [(option, count), ...]}``, plus ``total``."""
        pq = parse_query(query, strict=kw.get("strict"))
        words = " ".join(pq.keywords)
        if not words:
            raise ValueError("SparkFun facets need at least one search word")
        res = self._products(words, {}, 1, 1, with_aggs=True)
        return {"total": (res.get("total_count")),
                "attributes": {a["label"]: [(o["label"], o["count"]) for o in a.get("options") or []]
                               for a in res.get("aggregations") or [] if a.get("attribute_code") not in ("category_uid", "category_id")}}

    # -- single product ----------------------------------------------------------------------
    def getpart(self, code: str) -> Part:
        """One product by SKU (``SEN-19833``), with datasheet / schematic links read from its product page."""
        sku = code.strip()
        if not sku:
            raise ValueError("empty SKU")
        data = self._gql(f"{{ products(filter:{{sku:{{eq:{self._q(sku)}}}}}){{ items{{{_PRODUCT_FIELDS}}} }} }}")
        items = (data.get("products") or {}).get("items") or []
        if not items:
            raise NotFoundError(f"SparkFun has no product {sku!r}")
        part = _parse(items[0])
        if part.url:
            try:
                docs = self._documents(part.url)
            except PartsApiError:
                docs = []
            part.extra["documents"] = docs
            part.datasheet_url = next((d["url"] for d in docs if d["kind"] == "datasheet"), None)
        return part

    def _documents(self, url: str) -> list[dict[str, str]]:
        html = self.http.get_text(url)
        seen: list[dict[str, str]] = []
        for href in re.findall(r'href="([^"]+?\.pdf)(?:\?[^"]*)?"', html, re.I):
            href = href.replace("http://", "https://", 1)
            if not href.startswith("http"):
                href = BASE + (href if href.startswith("/") else "/" + href)
            low = href.lower()
            kind = "schematic" if "schematic" in low or "/sch" in low else "tutorial" if "learn_tutorials" in low else \
                "datasheet" if "datasheet" in low else "document"
            if href not in [d["url"] for d in seen]:
                seen.append({"url": href, "kind": kind})
        return seen

    def getpartprice(self, code: str, qty: Optional[int] = None) -> Union[list[PriceBreak], float]:
        breaks = self._item(code).price_breaks
        if qty is None:
            return breaks
        price = pick_price(breaks, qty)
        if price is None:
            raise NotFoundError(f"{code} has no price on SparkFun")
        return price

    getprice = getpartprice

    def _item(self, code: str) -> Part:
        """Like getpart without the (slower) product-page fetch."""
        data = self._gql(f"{{ products(filter:{{sku:{{eq:{self._q(code.strip())}}}}}){{ items{{{_PRODUCT_FIELDS}}} }} }}")
        items = (data.get("products") or {}).get("items") or []
        if not items:
            raise NotFoundError(f"SparkFun has no product {code!r}")
        return _parse(items[0])

    def getstock(self, code: str) -> Stock:
        s = self._item(code).stock
        assert s is not None
        return s

    def getattributes(self, code: str) -> dict[str, str]:
        """Always empty: SparkFun's API does not expose per-product attributes (use ``facets()`` / ``@"Name":value`` filters)."""
        self._item(code)
        return {}

    def getpics(self, code: str) -> list[str]:
        return self._item(code).image_urls

    def getpic(self, code: str, save_to: Optional[PathLike] = None, index: int = 0) -> Union[bytes, Path]:
        part = self._item(code)
        if not part.image_urls:
            raise NotFoundError(f"{code} has no picture on SparkFun")
        url = part.image_urls[min(index, len(part.image_urls) - 1)]
        data = self.http.get_bytes(url)
        ext = Path(url.split("?")[0]).suffix or ".jpg"
        return save_bytes(data, save_to, f"{safe_name(part.code)}{ext}") if save_to else data

    def getdatasheet(self, code: str, save_to: Optional[PathLike] = None) -> Union[bytes, Path]:
        part = self.getpart(code)
        if not part.datasheet_url:
            others = ", ".join(d["kind"] for d in part.extra.get("documents", [])) or "none"
            raise NotFoundError(f"{code}: no datasheet linked on its SparkFun page (other documents: {others})")
        data = self.http.get_bytes(part.datasheet_url)
        return save_bytes(data, save_to, f"{safe_name(part.code)}.pdf") if save_to else data

    def get3dmodel(self, code: str, fmt: str = "step", save_to: Optional[PathLike] = None) -> Model3D:
        raise NoModelError("SparkFun does not offer 3D models through its API")


_default = SparkFun()


def findpart(query: str, **kw: Any) -> list[Part]: return _default.findpart(query, **kw)
def facets(query: str, **kw: Any) -> dict[str, Any]: return _default.facets(query, **kw)
def getpart(code: str) -> Part: return _default.getpart(code)
def getpartprice(code: str, qty: Optional[int] = None): return _default.getpartprice(code, qty)
getprice = getpartprice
def getstock(code: str) -> Stock: return _default.getstock(code)
def getattributes(code: str) -> dict[str, str]: return _default.getattributes(code)
def getpics(code: str) -> list[str]: return _default.getpics(code)
def getpic(code: str, save_to: Optional[PathLike] = None, index: int = 0): return _default.getpic(code, save_to, index)
def getdatasheet(code: str, save_to: Optional[PathLike] = None): return _default.getdatasheet(code, save_to)
def get3dmodel(code: str, fmt: str = "step", save_to: Optional[PathLike] = None) -> Model3D: return _default.get3dmodel(code, fmt, save_to)
