"""TME (https://www.tme.eu) via the official TME API **v2**.

Needs a (free) API application from https://developers.tme.eu/ : its *token* and *application secret*. Applications
created after 14 May 2026 can only use v2 (the one implemented here); older applications use the legacy signed v1 API,
which this module does not support. Credentials come from the environment or the constructor::

    export TME_TOKEN=...   TME_SECRET=...          # optional: TME_COUNTRY=DE  TME_CURRENCY=EUR

OAuth2 client credentials are used (``POST /auth/token`` with Basic ``token:secret``; access tokens last 5 minutes and
are refreshed automatically).

    from partsapi import tme
    tme.findpart("op amp cat:\\"SMD operational amplifiers\\" is:instock", sort="price", limit=10)
    tme.findpart("resistor res:10k..100k tol:<=1% pkg:0603 is:instock", sort="price")
    tme.getpartprice("LM358DT", qty=100)
    tme.getstock("LM358DT")
    tme.getdatasheet("LM358DT", save_to="out/")

Prices follow your TME country: for Germany they are **gross including VAT** (``PriceBreak.incl_vat`` says so). Search
results carry no prices or stock, so every page of hits is completed with the prices/stock/parameters endpoints (up to
50 symbols per call). TME has no 3D models; ``get3dmodel`` matches the manufacturer part number in the LCSC/EasyEDA
library like the other distributors.
"""
from __future__ import annotations

import base64
import difflib
import os
import re
import time
from pathlib import Path
from typing import Any, Callable, Optional, Union

from . import easyeda as _easyeda
from ._cache import DiskCache, default_dir
from ._http import Http
from ._util import PathLike, safe_name, save_bytes, to_float, to_int
from .errors import AuthError, BlockedError, HttpError, NotFoundError, PartsApiError
from .filters import (ParsedQuery, PartFilter, build_filter, package_matches, parse_query,
                      reject_unsupported, sort_parts)
from .models import Model3D, Part, PriceBreak, Stock, pick_price

BASE = "https://api.tme.eu"
BATCH = 50  # the API's maximum number of symbols per call
CATEGORY_MAX_AGE = 7 * 24 * 3600.0
SORT_PROPERTIES = {"price": "PRICE_FIRST_QUANTITY", "code": "SYMBOL", "mpn": "ORIGINAL_SYMBOL"}
CARD_SKIP = {"text", "price", "stock", "attrs", "package", "datasheet"}  # not known from a search hit alone
_UNSET: Any = object()


def _abs(url: Optional[str]) -> Optional[str]:
    if not url:
        return None
    return "https:" + url if url.startswith("//") else url


def _env(name: str) -> Optional[str]:
    return os.environ.get(f"TME_{name}") or None


def _squash(s: str) -> str:
    return re.sub(r"[^a-z0-9]+", "", (s or "").casefold())


def _card(p: dict[str, Any], category: Optional[str] = None) -> Part:
    """A search hit / ``/products`` element as a :class:`Part` (no price, stock or parameters yet)."""
    symbols = p.get("manufacturer_symbols") or []
    photo = (p.get("assets") or {}).get("primary_photo") or {}
    images = [u for u in (_abs(photo.get("high_resolution")), _abs(photo.get("prime"))) if u]
    cat = p.get("category") or {}
    return Part(
        vendor="tme",
        code=p["symbol"],
        mpn=symbols[0] if symbols else None,
        manufacturer=(p.get("manufacturer") or {}).get("name"),
        description=p.get("description"),
        category=category or cat.get("name"),
        image_urls=images,
        extra={
            "ean": p.get("ean") or None,
            "category_id": cat.get("id"),
            "manufacturer_id": (p.get("manufacturer") or {}).get("id"),
            "manufacturer_symbols": symbols,
            "min_order_qty": to_int(p.get("minimal_amount")),
            "order_multiple": to_int(p.get("multiples")),
            "unit": (p.get("unit") or {}).get("short_name"),
            "packing": [{"type": e.get("id"), "quantity": to_int(e.get("amount"))} for e in (p.get("packing") or {}).get("elements") or []],
            "weight": p.get("weight"),
            "product_status": p.get("product_status") or [],
            "notification": p.get("notification"),
        },
        raw=p,
    )


def _apply_data(part: Part, d: dict[str, Any]) -> None:
    pr = d.get("prices") or {}
    cur = pr.get("currency") or "EUR"
    gross = pr.get("type") == "GROSS"
    rate = (to_float((pr.get("tax") or {}).get("rate")) or 0.0) / 100 or None
    rows = sorted(((int(e["amount"]), float(e["price"]), bool(e.get("special"))) for e in pr.get("elements") or [] if e.get("amount") and e.get("price") is not None))
    part.price_breaks = [PriceBreak(q, (rows[i + 1][0] - 1 if i + 1 < len(rows) else None), price, cur, gross, rate if gross else None)
                         for i, (q, price, _) in enumerate(rows)]
    part.extra["price_type"] = pr.get("type")
    part.extra["special_prices"] = [q for q, _, special in rows if special]
    qty = to_int(d.get("stock_quantity"))
    part.stock = Stock(vendor="tme", code=part.code, quantity=qty, in_stock=(qty or 0) > 0,
                       delivery=None, detail={"unit": (d.get("unit") or {}).get("short_name"), "deliveries": d.get("deliveries")})


def _apply_params(part: Part, d: dict[str, Any]) -> None:
    attrs = {e["name"]: ", ".join(str(v.get("value")) for v in e.get("values") or [] if v.get("value") is not None)
             for e in (d.get("parameters") or {}).get("elements") or [] if e.get("name")}
    part.attributes = attrs
    # passives list their size as "Case - inch" (0603) and "Case - mm" (1608); ICs use "Case" (SO8)
    part.package = attrs.get("Case") or attrs.get("Case - inch") or attrs.get("Package")
    if attrs.get("Case - mm"):
        part.extra["case_mm"] = attrs["Case - mm"]


class TME:
    def __init__(self, http: Optional[Http] = None, token: Optional[str] = None, secret: Optional[str] = None, *,
                 country: Optional[str] = None, currency: Optional[str] = None, cache_dir: Any = _UNSET) -> None:
        self.http = http or Http(min_interval=0.3)
        self.token = token or _env("TOKEN")
        self.secret = secret or _env("SECRET")
        self.country = country or _env("COUNTRY") or "DE"
        self.currency = currency or _env("CURRENCY") or ("EUR" if self.country in ("DE", "AT", "FR", "NL", "ES", "IT", "BE", "IE", "PT", "FI", "GR", "SK", "SI", "EE", "LV", "LT", "LU", "MT", "CY") else None)
        self.cache = DiskCache(default_dir() if cache_dir is _UNSET else (Path(cache_dir) if cache_dir else None))
        self._access: Optional[str] = None
        self._expires = 0.0
        self._tree: Optional[dict[str, list[Any]]] = None
        self._mfrs: Optional[list[dict[str, Any]]] = None
        self._easyeda = _easyeda.EasyEDA(self.http)

    def is_configured(self) -> bool:
        return bool(self.token and self.secret)

    # -- auth + transport --------------------------------------------------------------
    def _auth(self) -> str:
        if not self.is_configured():
            raise AuthError("TME needs TME_TOKEN and TME_SECRET (create an application at https://developers.tme.eu/ ; "
                            "the token is the long value, the secret the short one)")
        if self._access and time.monotonic() < self._expires:
            return self._access
        basic = base64.b64encode(f"{self.token}:{self.secret}".encode()).decode()
        try:
            resp = self.http.request("POST", f"{BASE}/auth/token", data={"grant_type": "client_credentials"}, headers={"Authorization": f"Basic {basic}"})
        except (BlockedError, HttpError) as exc:
            if exc.status not in (400, 401, 403):
                raise                                   # 429 / 5xx / ...: not a credentials problem
            raise AuthError("TME rejected the token/secret (only applications created after 14 May 2026 work with this v2 client; "
                            "check TME_TOKEN is the long value and TME_SECRET the short one)") from exc
        data = self.http._json(resp, "TME token endpoint")
        if "access_token" not in data:
            raise AuthError(f"TME token response had no access_token: {str(data)[:160]}")
        self._access = data["access_token"]
        self._expires = time.monotonic() + max(30.0, float(data.get("expires_in", 300)) - 30.0)
        return self._access

    def _get(self, path: str, params: list[tuple[str, Any]]) -> dict[str, Any]:
        for attempt in (0, 1):
            try:
                resp = self.http.request("GET", f"{BASE}{path}", params=params, headers={"Authorization": f"Bearer {self._auth()}", "Accept": "application/json"})
            except BlockedError as exc:
                if exc.status == 401 and attempt == 0:
                    self._access = None
                    continue
                if exc.status in (401, 403):
                    raise AuthError("TME refused the request (token expired, or the application lacks access to this endpoint)") from exc
                raise
            data = self.http._json(resp, path)
            if data.get("status") not in (None, "OK"):
                raise PartsApiError(f"TME error on {path}: {str(data)[:200]}")
            return data.get("data") or {}
        raise AuthError("TME authentication failed")  # pragma: no cover

    def _loc(self) -> list[tuple[str, Any]]:
        return [("country", self.country)]

    # -- reference data -----------------------------------------------------------------------
    def _categories(self) -> dict[str, list[Any]]:
        """``{id: [parent_id, name, product_count]}`` for all categories (about 400 KB, cached for 7 days)."""
        if self._tree is None:
            data = self.cache.get(f"tme_categories_{self.country}.json", CATEGORY_MAX_AGE)
            if data is None:
                els = self._get("/products/categories/list", self._loc()).get("elements") or []
                data = {str(e["id"]): [e.get("parent_id"), e.get("name") or "", e.get("products_count")] for e in els}
                self.cache.put(f"tme_categories_{self.country}.json", data)
            self._tree = data
        return self._tree

    def category_path(self, category_id: Any) -> Optional[str]:
        tree, names, cid, guard = self._categories(), [], str(category_id), 0
        while cid in tree and guard < 12:
            parent, name, _ = tree[cid]
            if name:
                names.append(name)
            if str(parent) == cid:
                break
            cid, guard = str(parent), guard + 1
        return " / ".join(reversed(names)) or None

    def _category_ids(self, operands: list[str], exact: bool) -> list[int]:
        tree = self._categories()
        key = lambda s: re.sub(r"[^a-z0-9]+", " ", (s or "").casefold()).strip()  # noqa: E731
        ids: list[int] = []
        for op in operands:
            want = key(op)
            same = [int(i) for i, (_, name, _) in tree.items() if name and key(name) == want]
            ids += same or ([int(i) for i, (_, name, _) in tree.items() if name and want and want in key(name)] if not exact else [])
        return list(dict.fromkeys(ids))

    def _suggest_categories(self, name: str) -> str:
        """" (similar: A, B, ...)" - spelling-close names first, then the biggest categories sharing a word."""
        tree = self._categories()
        names = {v[1].casefold(): v[1] for v in tree.values() if v[1]}
        close = [names[n] for n in difflib.get_close_matches(name.casefold(), list(names), n=3, cutoff=0.6)]
        words = [w.rstrip("s") for w in re.split(r"\W+", name.casefold()) if len(w) > 2]
        shared = [n for _, n in sorted(((v[2] or 0, v[1]) for v in tree.values() if v[1] and any(w in v[1].casefold() for w in words)), reverse=True)]
        picks = list(dict.fromkeys(close + shared))[:6]
        return f" (similar: {', '.join(picks)})" if picks else ""

    def manufacturers(self) -> list[dict[str, Any]]:
        if self._mfrs is None:
            self._mfrs = self._get("/products/manufacturers", self._loc()).get("elements") or []
        return self._mfrs

    # -- batch endpoints (<= 50 symbols per call) ------------------------------------------------
    def _batch(self, path: str, symbols: list[str], extra: Optional[list[tuple[str, Any]]] = None) -> dict[str, dict[str, Any]]:
        out: dict[str, dict[str, Any]] = {}
        for i in range(0, len(symbols), BATCH):
            chunk = symbols[i: i + BATCH]
            data = self._get(path, self._loc() + (extra or []) + [("symbols[]", s) for s in chunk])
            for e in data.get("elements") or []:
                out[e["symbol"]] = e
        return out

    def _data(self, symbols: list[str]) -> dict[str, dict[str, Any]]:
        extra: list[tuple[str, Any]] = [("scope[]", "prices"), ("scope[]", "stock")]
        if self.currency:
            extra.append(("currency", self.currency))
        return self._batch("/products/data", symbols, extra)

    def _hydrate(self, parts: list[Part], parameters: bool = True) -> None:
        syms = [p.code for p in parts]
        data = self._data(syms)
        params = self._batch("/products/parameters", syms) if parameters else {}
        for p in parts:
            if p.code in data:
                _apply_data(p, data[p.code])
            if p.code in params:
                _apply_params(p, params[p.code])

    def _documents(self, parts: list[Part]) -> None:
        files = self._batch("/products/files", [p.code for p in parts])
        for p in parts:
            f = files.get(p.code) or {}
            docs = [{"url": _abs(d.get("url")), "type": d.get("type"), "file_name": d.get("file_name"), "language": d.get("language")}
                    for d in (f.get("documents") or {}).get("elements") or []]
            p.extra["documents"] = docs
            ds = [d for d in docs if d["type"] == "DTE"]
            p.datasheet_url = next((d["url"] for d in ds if d["language"] == "EN"), ds[0]["url"] if ds else None)
            photos = (f.get("assets") or {}).get("primary_photo") or {}
            extra_imgs = [_abs(a.get("prime") or a.get("high_resolution")) for a in ((f.get("assets") or {}).get("additional") or {}).get("elements") or []]
            imgs = [u for u in (_abs(photos.get("high_resolution")), _abs(photos.get("prime"))) + tuple(extra_imgs) if u]
            if imgs:
                p.image_urls = list(dict.fromkeys(imgs))

    # -- search -------------------------------------------------------------------------------
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
        """Search TME with the full filter syntax of :mod:`partsapi.filters`.

        Sent to TME: the search words (2-40 characters), ``cat:`` (category id, includes sub-categories), ``mfr:`` when it
        names exactly one manufacturer, ``is:instock``, sorting by price (single unit) / code / mpn, and attribute filters
        (``res:10k..100k``, ``pkg:0603``, ``@"Name":value``) whose name matches one TME parameter. Everything else is checked
        client-side over up to ``max_pages`` pages of 100 hits. Every hit is completed with price, stock and parameters.
        Not available: ``is:rohs`` / ``lib:`` / ``is:preferred`` / ``has:3dmodel``.
        """
        if not 1 <= limit <= 1000:
            raise ValueError("limit must be 1..1000")
        pq = parse_query(query, qty=qty, strict=strict) if parse else ParsedQuery([query] if query.strip() else [], [], PartFilter(qty=qty))
        flt = pq.filter
        if where is not None:
            flt.merge(build_filter(where, qty))
        reject_unsupported(flt, "tme", flags={"preferred", "basic", "extended", "rohs"}, has={"3dmodel"}, library=True)
        queries = [k for k in pq.server_queries() if k.strip()]
        if not queries and any(n == "category" and not c.negate for n, c in flt.fields):
            queries = [""]  # TME accepts a category without a phrase
        if not queries:
            raise ValueError("TME search needs at least one search word or a cat:")
        for k in queries:
            if k and not 2 <= len(k) <= 40:
                raise ValueError(f"TME search phrases must be 2-40 characters (got {len(k)}: {k[:20]!r}...)")

        base: list[tuple[str, Any]] = self._loc()
        cat_ids: list[int] = []
        done_cat = done_mfr = False
        for name, cond in flt.fields:
            if cond.negate:
                continue
            if name == "category" and not done_cat and cond.op in ("auto", "eq"):
                cat_ids = self._category_ids(cond.operands, exact=cond.op == "eq")
                if not cat_ids:
                    raise ValueError(f"no TME category named {cond.operands[0]!r}" + self._suggest_categories(cond.operands[0]))
                if len(cat_ids) > 5:
                    names = ", ".join(self._categories()[str(i)][1] for i in cat_ids[:5])
                    raise ValueError(f"cat:{cond.operands[0]!r} matches {len(cat_ids)} TME categories ({names}, ...); use a more specific name or cat:=Exact")
                done_cat = True
            elif name == "manufacturer" and not done_mfr and cond.op in ("auto", "eq"):
                ids = [m["id"] for m in self.manufacturers() for o in cond.operands
                       if (m["name"].casefold() == o.casefold() if cond.op == "eq" else o.casefold() in m["name"].casefold())]
                if not ids:
                    return []
                if len(set(ids)) == 1:
                    base.append(("manufacturer_id", ids[0]))
                    done_mfr = True
        if "instock" in flt.flags or "stock" in flt.has:
            base.append(("filter[in_stock]", "true"))
        server_sort = SORT_PROPERTIES.get(sort) if isinstance(sort, str) and not (sort == "price" and qty != 1) else None
        if server_sort:
            base += [("sort[property]", server_sort), ("sort[direction]", "desc" if reverse else "asc")]
        wanted = limit if (not sort or server_sort) else 10**9
        page_size = 100 if (not flt.empty or sort) else min(limit, 100)

        found: dict[str, Part] = {}
        seen: set[str] = set()
        for kw in queries:
            for cid in (cat_ids or [None]):
                params = base + ([("phrase", kw)] if kw else []) + ([("category_id", cid)] if cid else [])
                resolved = self._resolve_params(params, flt)
                if resolved == "none":
                    continue
                params = params + (resolved or [])
                for pg in range(1, max_pages + 1):
                    data = self._get("/products/search", params + [("scope[]", "products"), ("scope[]", "counters"), ("limit", page_size), ("page", pg)])
                    cards = [_card(e) for e in (data.get("products") or {}).get("elements") or []]
                    fresh = [c for c in cards if c.code not in seen]
                    seen.update(c.code for c in fresh)
                    cheap = [c for c in fresh if flt.matches(c, skip=CARD_SKIP)]
                    if cheap:
                        self._hydrate(cheap)
                        if "datasheet" in flt.has:
                            self._documents(cheap)
                    for c in cheap:
                        if c.code not in found and flt.matches(c):
                            found[c.code] = c
                    pages = to_int(((data.get("counters") or {}).get("pages"))) or 0
                    if len(found) >= wanted or pg >= pages or not cards:
                        break
                if len(found) >= wanted:
                    break
            if len(found) >= wanted:
                break
        out = list(found.values())
        if sort:
            out = sort_parts(out, sort, reverse=reverse, qty=qty)
        for p in out[:limit]:
            if p.extra.get("category_id") and self._tree is not None:
                p.category = self.category_path(p.extra["category_id"]) or p.category
        return out[:limit]

    def _resolve_params(self, params: list[tuple[str, Any]], flt: PartFilter) -> Union[None, str, list[tuple[str, Any]]]:
        """Turn attribute / package conditions into TME ``parameters[n]`` filters, using the search's own facets.

        Conditions whose name matches several TME parameters (e.g. "voltage") or none, and negated ones, stay client-side.
        Returns the extra request parameters, ``"none"`` (no value can match) or ``None``."""
        todo: list[tuple[str, Any, bool, bool]] = [(n, c, e, False) for n, c, e in flt.attrs if not c.negate]
        todo += [("Case", c, True, True) for n, c in flt.fields if n == "package" and not c.negate]
        if not todo:
            return None
        data = self._get("/products/search", params + [("scope[]", "parameters"), ("limit", 1)])
        facets = (data.get("parameters") or {}).get("elements") or []
        out: list[tuple[str, Any]] = []
        k = 0
        by_name = {f.get("name"): f for f in facets if f.get("name")}
        for name, cond, exact, is_pkg in todo:
            if is_pkg:
                # sizes: "Case - inch" (0603) for passives, "Case" (SO8) for ICs; never "Case - mm", where 0603 is imperial 0201
                cands = []
                for fname in ("Case - inch", "Case"):
                    f = by_name.get(fname)
                    vals = [v for v in (f or {}).get("values") or [] if (v.get("products_count") or 0) > 0 and package_matches(cond, v.get("value"))]
                    if vals:
                        cands.append((f, vals))
                if not cands:
                    return "none" if any(n in by_name for n in ("Case - inch", "Case")) else None
                if len(cands) > 1:
                    continue  # matches in more than one facet: leave it to the client-side check
                f, ok = cands[0]
            else:
                chosen = flt.best_attr_name(by_name, name, exact)
                if chosen is None:
                    continue
                f = by_name[chosen]
                ok = [v for v in f.get("values") or [] if (v.get("products_count") or 0) > 0 and cond.test(v.get("value"))]
                if not ok:
                    return "none"
            out.append((f"parameters[{k}][id]", f["id"]))
            out += [(f"parameters[{k}][values][]", v["id"]) for v in ok]
            k += 1
        return out or None

    def facets(self, query: str, *, top: int = 15, **kw: Any) -> dict[str, Any]:
        """TME's own filter options for ``query`` (server-side): total, manufacturers, categories and every parameter with
        value counts. Accepts ``cat:`` / ``mfr:`` like :meth:`findpart`."""
        pq = parse_query(query, strict=kw.get("strict"))
        flt = pq.filter
        if kw.get("where") is not None:
            flt.merge(build_filter(kw["where"]))
        words = " ".join(pq.keywords)
        params: list[tuple[str, Any]] = self._loc()
        if words:
            params.append(("phrase", words))
        for name, cond in flt.fields:
            if name == "category" and not cond.negate:
                ids = self._category_ids(cond.operands, exact=cond.op == "eq")
                if len(ids) == 1:
                    params.append(("category_id", ids[0]))
        if len(params) == 1:
            raise ValueError("TME facets need a search word or a single cat:")
        data = self._get("/products/search", params + [("scope[]", "products"), ("scope[]", "parameters"), ("scope[]", "counters"), ("limit", 1)])
        counters = data.get("counters") or {}
        tree = self._categories()
        facs = (data.get("parameters") or {}).get("elements") or []
        by = {f.get("name"): [(v.get("value"), v.get("products_count")) for v in f.get("values") or [] if (v.get("products_count") or 0) > 0] for f in facs}
        return {
            "total": counters.get("count"),
            "manufacturers": sorted(by.pop("Manufacturer", []), key=lambda x: -(x[1] or 0))[:top],
            "categories": [(tree.get(str(c["id"]), [None, str(c["id"])])[1], c.get("products_count")) for c in (counters.get("categories") or {}).get("elements") or []][:top],
            "attributes": {k: sorted(v, key=lambda x: -(x[1] or 0))[:top] for k, v in by.items()},
        }

    # -- single part --------------------------------------------------------------------------
    def getpart(self, code: str) -> Part:
        """Everything TME has on a TME symbol (``LM358DT``) or an exact manufacturer part number."""
        code = code.strip()
        if not code:
            raise ValueError("empty part number")
        els: list[dict[str, Any]] = []
        for key in ("symbols[]", "mpns[]"):
            els = self._get("/products", self._loc() + [(key, code)]).get("elements") or []
            if els:
                break
        if not els:
            raise NotFoundError(f"TME has no part {code!r}")
        part = _card(els[0])
        part.category = self.category_path(part.extra["category_id"]) or part.category if part.extra.get("category_id") else part.category
        self._hydrate([part])
        self._documents([part])
        return part

    def _prices(self, code: str) -> Part:
        """Price + stock only (one call), for getprice / getstock."""
        sym = code.strip()
        data = self._data([sym])
        if sym not in data:
            sym = self.getpart(code).code  # an MPN, not a symbol
            data = self._data([sym])
        if sym not in data:
            raise NotFoundError(f"TME has no part {code!r}")
        part = Part(vendor="tme", code=sym)
        _apply_data(part, data[sym])
        return part

    def getpartprice(self, code: str, qty: Optional[int] = None) -> Union[list[PriceBreak], float]:
        """Price ladder (see ``PriceBreak.incl_vat`` for gross / net), or the unit price at ``qty``."""
        breaks = self._prices(code).price_breaks
        if qty is None:
            return breaks
        price = pick_price(breaks, qty)
        if price is None:
            raise NotFoundError(f"{code} has no price on TME")
        return price

    getprice = getpartprice

    def getstock(self, code: str) -> Stock:
        s = self._prices(code).stock
        assert s is not None
        return s

    def getattributes(self, code: str) -> dict[str, str]:
        sym = self.getpart(code).code
        part = Part(vendor="tme", code=sym)
        _apply_params(part, self._batch("/products/parameters", [sym]).get(sym) or {})
        return part.attributes

    def getpics(self, code: str) -> list[str]:
        return self.getpart(code).image_urls

    def getpic(self, code: str, save_to: Optional[PathLike] = None, index: int = 0) -> Union[bytes, Path]:
        part = self.getpart(code)
        if not part.image_urls:
            raise NotFoundError(f"{code} has no picture on TME")
        url = part.image_urls[min(index, len(part.image_urls) - 1)]
        data = self.http.get_bytes(url)
        return save_bytes(data, save_to, f"{safe_name(part.code)}.jpg") if save_to else data

    def getdatasheet(self, code: str, save_to: Optional[PathLike] = None) -> Union[bytes, Path]:
        """The datasheet PDF (English if available)."""
        part = self.getpart(code)
        if not part.datasheet_url:
            raise NotFoundError(f"{code} has no datasheet on TME")
        data = self.http.get_bytes(part.datasheet_url)
        return save_bytes(data, save_to, f"{safe_name(part.code)}.pdf") if save_to else data

    def get3dmodel(self, code: str, fmt: str = "step", save_to: Optional[PathLike] = None) -> Model3D:
        """TME's API has no 3D models; the manufacturer part number is matched exactly against the LCSC/EasyEDA library."""
        mpn = self.getpart(code).mpn
        if not mpn:
            raise PartsApiError(f"{code}: TME lists no manufacturer part number")
        return self._easyeda.get3dmodel_by_mpn(mpn, fmt, save_to)


_default = TME()


def is_configured() -> bool: return _default.is_configured()
def findpart(query: str, **kw: Any) -> list[Part]: return _default.findpart(query, **kw)
def facets(query: str, **kw: Any) -> dict[str, Any]: return _default.facets(query, **kw)
def categories() -> dict[str, list[Any]]: return _default._categories()
def getpart(code: str) -> Part: return _default.getpart(code)
def getpartprice(code: str, qty: Optional[int] = None): return _default.getpartprice(code, qty)
getprice = getpartprice
def getstock(code: str) -> Stock: return _default.getstock(code)
def getattributes(code: str) -> dict[str, str]: return _default.getattributes(code)
def getpics(code: str) -> list[str]: return _default.getpics(code)
def getpic(code: str, save_to: Optional[PathLike] = None, index: int = 0): return _default.getpic(code, save_to, index)
def getdatasheet(code: str, save_to: Optional[PathLike] = None): return _default.getdatasheet(code, save_to)
def get3dmodel(code: str, fmt: str = "step", save_to: Optional[PathLike] = None) -> Model3D: return _default.get3dmodel(code, fmt, save_to)
