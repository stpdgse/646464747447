"""Flask app: a small JSON API over the vendor modules plus a single-page UI (``static/``).

Security model
--------------
* Vendor credentials stay on the server; nothing secret is ever sent to the browser.
* Every ``/api/*`` call except ``/api/health`` needs the access key from ``WEBUI_PASSWORD`` (header ``X-Access-Key``).
  If no password is configured the API only answers on localhost (never when running on Vercel or behind a proxy), so a
  deployment without a password is inert, not open.
* The browser never supplies URLs: requests name a vendor from a fixed list and a part code, so the server cannot be
  pointed at arbitrary hosts.
"""
from __future__ import annotations

import hmac
import json
import os
import re
import time
from collections import defaultdict, deque
from functools import wraps
from pathlib import Path
from typing import Any, Callable, Optional

from flask import Flask, Response, request, send_from_directory

from .. import adafruit, digikey, jlc, lcsc, mouser, parts, reichelt, sparkfun, tme
from ..errors import AuthError, BlockedError, NoModelError, NotFoundError, ParseError, PartsApiError
from ..models import Part

MODULES = {"jlc": jlc, "lcsc": lcsc, "reichelt": reichelt, "digikey": digikey, "mouser": mouser, "tme": tme,
           "adafruit": adafruit, "sparkfun": sparkfun}
# id -> (display name, needs credentials, note shown in the UI)
INFO: dict[str, tuple[str, bool, str]] = {
    "jlc": ("JLCPCB", False, "Basic / extended assembly library, USD. Full numeric attribute filters."),
    "lcsc": ("LCSC", False, "Prices in USD. Keyword search goes through the EasyEDA catalogue."),
    "reichelt": ("Reichelt", False, "German shop. EUR gross incl. VAT; attribute names are German."),
    "digikey": ("DigiKey", True, "Official API. Parametric filters work inside one leaf category."),
    "mouser": ("Mouser", True, "Official API. No technical attributes: put values in the search words."),
    "tme": ("TME", True, "Official API v2. Parametric filters, real stock; gross prices for Germany."),
    "adafruit": ("Adafruit", False, "Hobby shop. Searches a cached copy of the catalogue (first search downloads ~8 MB)."),
    "sparkfun": ("SparkFun", False, "Hobby shop. In/out of stock only, no attribute data; facet filters via @\"Name\":value."),
}
SORTS = {"price", "stock", "mpn", "code", "manufacturer", "package"}
MAX_MODEL_BYTES = 4_200_000  # serverless response bodies are capped at ~4.5 MB
STATIC = Path(__file__).parent / "static"

_failures: dict[str, deque] = defaultdict(lambda: deque(maxlen=20))


class ApiError(Exception):
    def __init__(self, message: str, status: int = 400, kind: str = "request") -> None:
        super().__init__(message)
        self.status, self.kind = status, kind


def _json(payload: Any, status: int = 200) -> Response:
    return Response(json.dumps(payload, ensure_ascii=False, default=str), status=status, mimetype="application/json")


def _vendor_id() -> str:
    v = request.args.get("vendor", "")
    if v not in MODULES:
        raise ApiError(f"unknown vendor {v!r}; use one of: {', '.join(MODULES)}")
    return v


def _int(name: str, default: int, lo: int, hi: int) -> int:
    raw = request.args.get(name)
    if raw in (None, ""):
        return default
    try:
        n = int(raw)
    except ValueError:
        raise ApiError(f"{name} must be a whole number") from None
    if not lo <= n <= hi:
        raise ApiError(f"{name} must be between {lo} and {hi}")
    return n


# --------------------------------------------------------------------------------------- serialisation
def _price_at(part: Part, qty: int) -> tuple[Optional[float], Optional[str], Optional[bool], Optional[float]]:
    br = part.price_breaks[0] if part.price_breaks else None
    return part.price_for(qty), (br.currency if br else None), (br.incl_vat if br else None), (br.vat_rate if br else None)


def _row(part: Part, vendor: str, qty: int) -> dict[str, Any]:
    price, cur, incl, vat = _price_at(part, qty)
    s = part.stock
    return {
        "vendor": vendor, "code": part.code, "mpn": part.mpn, "manufacturer": part.manufacturer, "description": part.description,
        "category": part.category, "package": part.package, "url": part.url, "image": part.image_urls[0] if part.image_urls else None,
        "price": price, "currency": cur, "incl_vat": incl, "vat_rate": vat, "price_qty": qty,
        "stock_qty": s.quantity if s else None, "in_stock": s.in_stock if s else None, "delivery": s.delivery if s else None,
        "library": part.extra.get("library_type"), "preferred": part.extra.get("preferred"), "rohs": part.extra.get("rohs"),
        "discontinued": part.extra.get("discontinued"), "has_price": bool(part.price_breaks),
    }


def _detail(part: Part, vendor: str, qty: int) -> dict[str, Any]:
    d = _row(part, vendor, qty)
    extra = {k: v for k, v in part.extra.items() if k not in ("description_html", "variations", "symbol_svg", "footprint_svg") and v not in (None, "", [], {})}
    d.update(
        images=part.image_urls[:8], datasheet=part.datasheet_url, attributes=part.attributes, extra=extra,
        documents=part.extra.get("documents") or [],
        ladder=[{"min": b.min_qty, "max": b.max_qty, "price": b.unit_price, "currency": b.currency, "incl_vat": b.incl_vat} for b in part.price_breaks],
        stock_detail=part.stock.detail if part.stock else None,
        variations=part.extra.get("variations") if vendor == "digikey" else None,
    )
    return d


def _groups(vendor: str, f: dict[str, Any]) -> dict[str, Any]:
    """Normalise the very different ``facets()`` results into ``[{title, items: [[label, count, token], ...]}]``."""
    exact_pkg = vendor == "jlc"

    def tok(kind: str, label: Any, name: str = "") -> Optional[str]:
        if not isinstance(label, str) or '"' in label or not label.strip():
            return None
        return {"mfr": f'mfr:="{label}"' if vendor in ("jlc", "digikey", "tme") else f'mfr:"{label}"',
                "pkg": f'pkg:="{label}"' if exact_pkg else f'pkg:"{label}"', "cat": f'cat:"{label}"',
                "attr": f'@"{name}":"{label}"' if '"' not in name else None}[kind]

    def items(pairs: Any, kind: str, name: str = "") -> list[list[Any]]:
        out = []
        for p in list(pairs or [])[:40]:
            label, count = (p[0], p[1]) if isinstance(p, (list, tuple)) else (p.get("name"), p.get("count"))
            out.append([label, count, tok(kind, label, name)])
        return out

    groups: list[dict[str, Any]] = []
    for key, title, kind in (("manufacturers", "Manufacturers", "mfr"), ("brands", "Brands", "mfr"), ("packages", "Packages", "pkg"),
                             ("categories", "Categories", "cat"), ("packaging", "Packaging", None), ("status", "Status", None),
                             ("series", "Series", None), ("libraries", "Library", None)):
        val = f.get(key)
        if val:
            groups.append({"title": title, "items": items(val if not (key == "categories" and val and isinstance(val[0], dict)) else
                                                         [{"name": c["name"], "count": c["count"]} for c in val], kind or "x")})
            if not kind:
                for it in groups[-1]["items"]:
                    it[2] = None
    for name, vals in (f.get("attributes") or {}).items():
        groups.append({"title": name, "items": items(vals, "attr", name)})
    if f.get("counts"):
        groups.append({"title": "Counts", "items": [[k.replace("_", " "), v, None] for k, v in f["counts"].items() if v is not None]})
    return {"total": f.get("total"), "groups": [g for g in groups if g["items"]], "price_range": f.get("price_range")}


# --------------------------------------------------------------------------------------------- app
def create_app() -> Flask:
    app = Flask(__name__, static_folder=None)
    app.json.ensure_ascii = False  # type: ignore[attr-defined]

    def password() -> str:
        return os.environ.get("WEBUI_PASSWORD", "")

    def local_only_ok() -> bool:
        return not os.environ.get("VERCEL") and request.remote_addr in ("127.0.0.1", "::1") and "X-Forwarded-For" not in request.headers

    @app.before_request
    def gate() -> Optional[Response]:
        if not request.path.startswith("/api/") or request.path == "/api/health":
            return None
        ip = request.headers.get("X-Forwarded-For", request.remote_addr or "?").split(",")[0].strip()
        recent = [t for t in _failures[ip] if time.time() - t < 300]
        if len(recent) >= 10:
            return _json({"error": "too many wrong access keys; wait a few minutes", "kind": "auth"}, 429)
        pw = password()
        if not pw:
            if local_only_ok():
                return None
            return _json({"error": "WEBUI_PASSWORD is not set on this server, so the API only answers on localhost", "kind": "auth"}, 503)
        given = request.headers.get("X-Access-Key", "")
        if hmac.compare_digest(given.encode(), pw.encode()):
            return None
        _failures[ip].append(time.time())
        return _json({"error": "access key required" if not given else "wrong access key", "kind": "auth"}, 401)

    @app.after_request
    def headers(resp: Response) -> Response:
        resp.headers["X-Content-Type-Options"] = "nosniff"
        resp.headers["Referrer-Policy"] = "no-referrer"
        resp.headers["Content-Security-Policy"] = ("default-src 'self'; img-src * data:; style-src 'self'; script-src 'self'; "
                                                   "connect-src 'self'; frame-ancestors 'none'; base-uri 'none'; form-action 'self'")
        if request.path.startswith("/api/"):
            resp.headers["Cache-Control"] = "no-store"
        return resp

    def api(fn: Callable[..., Any]) -> Callable[..., Response]:
        @wraps(fn)
        def wrapper(*a: Any, **k: Any) -> Response:
            try:
                return _json(fn(*a, **k))
            except ApiError as e:
                return _json({"error": str(e), "kind": e.kind}, e.status)
            except ValueError as e:          # unsupported filter, bad argument: the message is written for users
                return _json({"error": str(e), "kind": "filter"}, 400)
            except AuthError as e:
                return _json({"error": str(e), "kind": "credentials"}, 424)
            except NotFoundError as e:
                return _json({"error": str(e), "kind": "notfound"}, 404)
            except BlockedError as e:
                return _json({"error": f"the vendor refused or rate-limited the request: {e}", "kind": "vendor"}, 502)
            except (ParseError, PartsApiError) as e:
                return _json({"error": str(e), "kind": "vendor"}, 502)
            except Exception:  # noqa: BLE001 - never leak a traceback to the browser
                app.logger.exception("unhandled error in %s", request.path)
                return _json({"error": "internal error (see the server log)", "kind": "internal"}, 500)
        return wrapper

    @app.get("/")
    def index() -> Response:
        return send_from_directory(STATIC, "index.html")

    @app.get("/static/<path:name>")
    def static_files(name: str) -> Response:
        return send_from_directory(STATIC, name)

    @app.get("/api/health")
    def health() -> Response:
        return _json({"ok": True, "password_required": bool(password())})

    @app.get("/api/vendors")
    @api
    def vendors() -> Any:
        return {"vendors": [{"id": vid, "name": INFO[vid][0], "needs_key": INFO[vid][1], "configured": (MODULES[vid].is_configured() if INFO[vid][1] else True),
                             "note": INFO[vid][2]} for vid in MODULES]}

    @app.get("/api/search")
    @api
    def search() -> Any:
        vid = _vendor_id()
        q, where = request.args.get("q", "").strip(), request.args.get("where", "").strip()
        if not q and not where:
            raise ApiError("type something to search for")
        if len(q) > 300 or len(where) > 300:
            raise ApiError("query too long")
        sort = request.args.get("sort", "") or None
        if sort and sort not in SORTS:
            raise ApiError(f"sort must be one of: {', '.join(sorted(SORTS))}")
        qty, limit = _int("qty", 1, 1, 10_000_000), _int("limit", 20, 1, 100)
        kw: dict[str, Any] = {"limit": limit, "qty": qty, "sort": sort, "reverse": request.args.get("reverse") == "1"}
        if where:
            kw["where"] = where
        if vid == "adafruit" and request.args.get("discontinued") == "1":
            kw["include_discontinued"] = True
        t0 = time.perf_counter()
        found = MODULES[vid].findpart(q, **kw)
        return {"vendor": vid, "count": len(found), "limit": limit, "ms": int((time.perf_counter() - t0) * 1000),
                "parts": [_row(p, vid, qty) for p in found]}

    @app.get("/api/part")
    @api
    def part() -> Any:
        vid, code = _vendor_id(), request.args.get("code", "").strip()
        if not code or len(code) > 120:
            raise ApiError("code is required")
        qty = _int("qty", 1, 1, 10_000_000)
        return _detail(MODULES[vid].getpart(code), vid, qty)

    @app.get("/api/facets")
    @api
    def facets() -> Any:
        vid, q = _vendor_id(), request.args.get("q", "").strip()
        if not q:
            raise ApiError("type something to search for first")
        where = request.args.get("where", "").strip()
        return _groups(vid, MODULES[vid].facets(q, **({"where": where} if where and vid != "jlc" else {})))

    @app.get("/api/prices")
    @api
    def prices() -> Any:
        vid, q = _vendor_id(), request.args.get("q", "").strip()
        if not q or len(q) > 120:
            raise ApiError("enter a manufacturer part number")
        return {"vendor": vid, "rows": parts.prices(q, _int("qty", 1, 1, 10_000_000), vendors=(vid,))}

    @app.get("/api/model3d")
    def model3d() -> Response:
        try:
            vid, code = _vendor_id(), request.args.get("code", "").strip()
            fmt = request.args.get("fmt", "step")
            if fmt not in ("step", "obj") or not code or len(code) > 120:
                raise ApiError("fmt must be step or obj and code is required")
            m = MODULES[vid].get3dmodel(code, fmt)
        except ApiError as e:
            return _json({"error": str(e), "kind": e.kind}, e.status)
        except NoModelError as e:
            return _json({"error": str(e), "kind": "notfound"}, 404)
        except (NotFoundError, AuthError, PartsApiError, ValueError) as e:
            return _json({"error": str(e), "kind": "vendor"}, 404 if isinstance(e, NotFoundError) else 502)
        if len(m.data) > MAX_MODEL_BYTES:
            return _json({"error": f"the {fmt.upper()} file is {len(m.data) // 1000} kB, too big for this server; try the other format", "kind": "size"}, 413)
        safe = re.sub(r"[^A-Za-z0-9._-]+", "_", m.filename)
        return Response(m.data, mimetype="application/octet-stream", headers={
            "Content-Disposition": f'attachment; filename="{safe}"', "X-Model-Source": m.source_code, "Cache-Control": "no-store"})

    return app


app = create_app()
