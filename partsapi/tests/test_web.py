import re
from pathlib import Path

import pytest

pytest.importorskip("flask")

from partsapi.errors import AuthError, BlockedError, NoModelError, NotFoundError, PartsApiError  # noqa: E402
from partsapi.models import Model3D, Part, PriceBreak, Stock  # noqa: E402
from partsapi.web import app as webapp  # noqa: E402

KEY = "correct horse"
STATIC = Path(webapp.__file__).parent / "static"


def make_part(code="C1", **kw):
    return Part(vendor="jlc", code=code, mpn=kw.get("mpn", "MPN-" + code), manufacturer="ACME", description="10k resistor", category="Resistors", package="0603",
                url="https://example.test/p/" + code, image_urls=["https://example.test/i.jpg"], datasheet_url="https://example.test/ds.pdf",
                price_breaks=[PriceBreak(1, 9, 0.5, "USD"), PriceBreak(10, None, 0.25, "USD")], stock=Stock("jlc", code, 120, True),
                attributes={"Resistance": "10kΩ"}, extra={"library_type": "basic", "rohs": True, "documents": [{"url": "https://example.test/ds.pdf", "kind": "datasheet"}],
                                                          "description_html": "<b>x</b>", "note": "n"})


class Fake:
    """Stands in for a vendor module and records how it was called."""

    def __init__(self, **over):
        self.calls = []
        self.find = over.get("find", lambda q, **kw: [make_part("C1"), make_part("C2")])
        self.get = over.get("get", lambda code: make_part(code))
        self.fac = over.get("facets", lambda q, **kw: {"total": 5, "manufacturers": [("ACME", 3)], "packages": [("0603", 4)], "attributes": {"Resistance": [("10kΩ", 2)]}})
        self.model = over.get("model", lambda code, fmt, save_to=None: Model3D(data=b"ISO-10303-21;", format=fmt, filename=f"{code}.{fmt}", source_code="C9418", uuid="u"))
        self.configured = over.get("configured", True)

    def findpart(self, q, **kw):
        self.calls.append(("findpart", q, kw)); return self.find(q, **kw)

    def getpart(self, code): return self.get(code)
    def facets(self, q, **kw): self.calls.append(("facets", q, kw)); return self.fac(q, **kw)
    def get3dmodel(self, code, fmt="step", save_to=None): return self.model(code, fmt)
    def is_configured(self): return self.configured


@pytest.fixture
def web(monkeypatch):
    fake = Fake()
    monkeypatch.setattr(webapp, "MODULES", {k: fake for k in webapp.MODULES})
    monkeypatch.delenv("VERCEL", raising=False)
    monkeypatch.setenv("WEBUI_PASSWORD", KEY)
    webapp._failures.clear()
    client = webapp.create_app().test_client()
    client.fake = fake
    client.h = {"X-Access-Key": KEY}
    return client


def get(web, path, **kw):
    return web.get(path, headers=web.h, **kw)


# ------------------------------------------------------------------ access control
def test_health_is_public_and_tells_whether_a_password_is_needed(web, monkeypatch):
    r = web.get("/api/health")
    assert r.status_code == 200 and r.json == {"ok": True, "password_required": True}
    monkeypatch.delenv("WEBUI_PASSWORD")
    assert web.get("/api/health").json["password_required"] is False


def test_api_needs_the_access_key(web):
    assert web.get("/api/vendors").status_code == 401
    r = web.get("/api/vendors", headers={"X-Access-Key": "nope"})
    assert r.status_code == 401 and r.json["kind"] == "auth" and "wrong" in r.json["error"]
    assert web.get("/api/vendors", headers=web.h).status_code == 200
    assert web.get("/api/search?vendor=jlc&q=x").status_code == 401 and web.get("/api/model3d?vendor=jlc&code=C1").status_code == 401


def test_repeated_wrong_keys_are_throttled(web):
    for _ in range(10):
        assert web.get("/api/vendors", headers={"X-Access-Key": "bad"}).status_code == 401
    assert web.get("/api/vendors", headers={"X-Access-Key": "bad"}).status_code == 429
    assert web.get("/api/vendors", headers=web.h).status_code == 429          # even the right key waits out the block


def test_without_a_password_only_localhost_is_served(web, monkeypatch):
    monkeypatch.delenv("WEBUI_PASSWORD")
    assert web.get("/api/vendors").status_code == 200                           # the test client is 127.0.0.1
    r = web.get("/api/vendors", headers={"X-Forwarded-For": "203.0.113.9"})     # behind a proxy: not local
    assert r.status_code == 503 and "WEBUI_PASSWORD" in r.json["error"]
    monkeypatch.setenv("VERCEL", "1")
    assert web.get("/api/vendors").status_code == 503                           # never open on Vercel
    assert web.get("/api/health").status_code == 200


# ------------------------------------------------------------------ vendors, static, headers
def test_vendors_list(web):
    web.fake.configured = False
    v = get(web, "/api/vendors").json["vendors"]
    assert [x["id"] for x in v] == ["jlc", "lcsc", "reichelt", "digikey", "mouser", "tme", "adafruit", "sparkfun"]
    by = {x["id"]: x for x in v}
    assert by["jlc"]["configured"] is True and by["digikey"]["configured"] is False and by["digikey"]["needs_key"] is True
    assert all(x["name"] and x["note"] for x in v)


def test_security_headers_and_static_files(web):
    r = web.get("/")
    assert r.status_code == 200 and b"Parts Search" in r.data
    csp = r.headers["Content-Security-Policy"]
    assert "script-src 'self'" in csp and "unsafe-inline" not in csp and "frame-ancestors 'none'" in csp
    assert r.headers["X-Content-Type-Options"] == "nosniff" and r.headers["Referrer-Policy"] == "no-referrer"
    assert web.get("/static/app.js").status_code == 200 and web.get("/static/style.css").status_code == 200
    assert web.get("/static/../app.py").status_code == 404 and web.get("/static/%2e%2e/app.py").status_code == 404
    assert get(web, "/api/vendors").headers["Cache-Control"] == "no-store"


def test_page_has_no_inline_script_or_style_that_the_csp_would_block():
    html = (STATIC / "index.html").read_text()
    assert not re.search(r"<script(?![^>]*\bsrc=)", html) and " style=" not in html and not re.search(r"\son[a-z]+=", html)
    assert "<style" not in html


# ------------------------------------------------------------------ search
def test_search_returns_rows_priced_at_the_quantity(web):
    r = get(web, "/api/search?vendor=jlc&q=resistor&qty=10&limit=5&sort=price&reverse=1&where=is:instock")
    assert r.status_code == 200
    j = r.json
    assert j["vendor"] == "jlc" and j["count"] == 2 and j["limit"] == 5 and isinstance(j["ms"], int)
    p = j["parts"][0]
    assert (p["code"], p["mpn"], p["price"], p["currency"], p["price_qty"], p["stock_qty"], p["in_stock"], p["library"], p["rohs"]) == ("C1", "MPN-C1", 0.25, "USD", 10, 120, True, "basic", True)
    assert p["image"] == "https://example.test/i.jpg" and "raw" not in p and "attributes" not in p
    name, q, kw = web.fake.calls[-1]
    assert (q, kw["limit"], kw["qty"], kw["sort"], kw["reverse"], kw["where"]) == ("resistor", 5, 10, "price", True, "is:instock")


def test_search_defaults_and_where_only(web):
    get(web, "/api/search?vendor=lcsc&q=lm358")
    _, _, kw = web.fake.calls[-1]
    assert kw == {"limit": 20, "qty": 1, "sort": None, "reverse": False}
    assert get(web, "/api/search?vendor=lcsc&where=res:10k").status_code == 200       # filters alone are allowed through


def test_discontinued_flag_is_only_passed_to_adafruit(web):
    get(web, "/api/search?vendor=adafruit&q=dht22&discontinued=1")
    assert web.fake.calls[-1][2]["include_discontinued"] is True
    get(web, "/api/search?vendor=jlc&q=dht22&discontinued=1")
    assert "include_discontinued" not in web.fake.calls[-1][2]


@pytest.mark.parametrize("query,fragment", [
    ("vendor=bogus&q=x", "unknown vendor"), ("q=x", "unknown vendor"), ("vendor=jlc", "type something"), ("vendor=jlc&q=%20", "type something"),
    ("vendor=jlc&q=x&sort=colour", "sort must be"), ("vendor=jlc&q=x&limit=0", "limit must be"), ("vendor=jlc&q=x&limit=101", "limit must be"),
    ("vendor=jlc&q=x&limit=abc", "whole number"), ("vendor=jlc&q=x&qty=0", "qty must be"), ("vendor=jlc&q=" + "a" * 301, "too long"),
])
def test_search_validation(web, query, fragment):
    r = get(web, "/api/search?" + query)
    assert r.status_code == 400 and fragment in r.json["error"]
    assert not web.fake.calls


@pytest.mark.parametrize("exc,status,kind", [
    (ValueError("mouser does not support is:preferred"), 400, "filter"), (AuthError("needs DIGIKEY_CLIENT_ID"), 424, "credentials"),
    (NotFoundError("no such part"), 404, "notfound"), (BlockedError("rate limited", 429), 502, "vendor"), (PartsApiError("layout changed"), 502, "vendor"),
])
def test_vendor_errors_become_clear_json(web, exc, status, kind):
    def boom(q, **kw): raise exc
    web.fake.find = boom
    r = get(web, "/api/search?vendor=jlc&q=x")
    assert r.status_code == status and r.json["kind"] == kind and str(exc) in r.json["error"]


def test_unexpected_errors_do_not_leak_details(web):
    def boom(q, **kw): raise RuntimeError("secret internal path /srv/app token=abc123")
    web.fake.find = boom
    r = get(web, "/api/search?vendor=jlc&q=x")
    assert r.status_code == 500 and r.json["kind"] == "internal" and "abc123" not in r.get_data(as_text=True) and "RuntimeError" not in r.get_data(as_text=True)


# ------------------------------------------------------------------ part detail
def test_part_detail(web):
    j = get(web, "/api/part?vendor=jlc&code=C25804&qty=10").json
    assert j["code"] == "C25804" and j["datasheet"] == "https://example.test/ds.pdf" and j["attributes"] == {"Resistance": "10kΩ"}
    assert j["ladder"] == [{"min": 1, "max": 9, "price": 0.5, "currency": "USD", "incl_vat": None}, {"min": 10, "max": None, "price": 0.25, "currency": "USD", "incl_vat": None}]
    assert j["price"] == 0.25 and j["documents"][0]["kind"] == "datasheet"
    assert "description_html" not in j["extra"] and j["extra"]["note"] == "n"          # bulky / internal fields are dropped


def test_part_validation_and_not_found(web):
    assert get(web, "/api/part?vendor=jlc").status_code == 400
    assert get(web, "/api/part?vendor=jlc&code=" + "x" * 121).status_code == 400
    web.fake.get = lambda code: (_ for _ in ()).throw(NotFoundError("no part"))
    assert get(web, "/api/part?vendor=jlc&code=Z").status_code == 404


# ------------------------------------------------------------------ facets
def test_facets_are_normalised_into_clickable_filters(web):
    j = get(web, "/api/facets?vendor=digikey&q=resistor").json
    titles = {g["title"]: g["items"] for g in j["groups"]}
    assert j["total"] == 5
    assert titles["Manufacturers"] == [["ACME", 3, 'mfr:="ACME"']] and titles["Packages"] == [["0603", 4, 'pkg:"0603"']]
    assert titles["Resistance"] == [["10kΩ", 2, '@"Resistance":"10kΩ"']]
    assert get(web, "/api/facets?vendor=jlc&q=resistor").json["groups"][1]["items"][0][2] == 'pkg:="0603"'      # JLCPCB filters exact package names


def test_facet_tokens_skip_values_that_cannot_be_quoted(web):
    web.fake.fac = lambda q, **kw: {"packages": [('1" x 1"', 3), ("ok", 1)], "categories": [{"name": "Sensors", "count": 9, "children": []}], "counts": {"basic": 185, "none": None}}
    groups = {g["title"]: g["items"] for g in get(web, "/api/facets?vendor=sparkfun&q=x").json["groups"]}
    assert groups["Packages"] == [['1" x 1"', 3, None], ["ok", 1, 'pkg:"ok"']]
    assert groups["Categories"] == [["Sensors", 9, 'cat:"Sensors"']] and groups["Counts"] == [["basic", 185, None]]


def test_facets_need_a_query_and_forward_where(web):
    assert get(web, "/api/facets?vendor=tme").status_code == 400
    get(web, "/api/facets?vendor=tme&q=x&where=is:instock")
    assert web.fake.calls[-1][2] == {"where": "is:instock"}
    get(web, "/api/facets?vendor=jlc&q=x&where=is:instock")
    assert web.fake.calls[-1][2] == {}                                          # JLCPCB facets take no filter expression


# ------------------------------------------------------------------ prices and 3D
def test_prices_calls_the_cross_vendor_helper_for_one_vendor(web, monkeypatch):
    seen = {}
    monkeypatch.setattr(webapp.parts, "prices", lambda q, qty, vendors: seen.update(q=q, qty=qty, vendors=vendors) or [{"vendor": vendors[0], "unit_price": 0.1}])
    j = get(web, "/api/prices?vendor=tme&q=LM358DR&qty=10").json
    assert seen == {"q": "LM358DR", "qty": 10, "vendors": ("tme",)} and j["rows"][0]["unit_price"] == 0.1
    assert get(web, "/api/prices?vendor=tme").status_code == 400


def test_model3d_download(web):
    r = get(web, "/api/model3d?vendor=jlc&code=C2913204&fmt=step")
    assert r.status_code == 200 and r.data == b"ISO-10303-21;" and r.headers["X-Model-Source"] == "C9418"
    assert r.headers["Content-Disposition"] == 'attachment; filename="C2913204.step"'
    assert get(web, "/api/model3d?vendor=jlc&code=C1&fmt=stl").status_code == 400
    assert get(web, "/api/model3d?vendor=jlc&fmt=step").status_code == 400


def test_model3d_errors(web):
    web.fake.model = lambda code, fmt: (_ for _ in ()).throw(NoModelError("no 3D model"))
    r = get(web, "/api/model3d?vendor=jlc&code=C1")
    assert r.status_code == 404 and "no 3D model" in r.json["error"]
    web.fake.model = lambda code, fmt: Model3D(data=b"x" * (webapp.MAX_MODEL_BYTES + 1), format=fmt, filename="big.step", source_code="C1", uuid="u")
    r = get(web, "/api/model3d?vendor=jlc&code=C1")
    assert r.status_code == 413 and r.json["kind"] == "size" and "try the other format" in r.json["error"]
    web.fake.model = lambda code, fmt: Model3D(data=b"ISO-10303-21;", format=fmt, filename='we"ird name.step', source_code="C1", uuid="u")
    assert get(web, "/api/model3d?vendor=jlc&code=C1").headers["Content-Disposition"] == 'attachment; filename="we_ird_name.step"'
    web.fake.model = lambda code, fmt: (_ for _ in ()).throw(AuthError("creds"))
    assert get(web, "/api/model3d?vendor=jlc&code=C1").status_code == 502


# ------------------------------------------------------------------ Vercel path restoration
def test_rewritten_requests_get_their_original_path_back(web):
    from werkzeug.test import Client

    from partsapi.web.wsgi import RestoreOriginalPath
    client = Client(RestoreOriginalPath(webapp.create_app().wsgi_app))
    r = client.get("/api/index?__path=api/health&keep=1")
    assert r.status_code == 200 and r.get_json() == {"ok": True, "password_required": True}
    assert client.get("/api/index?__path=").status_code == 200 and b"Parts Search" in client.get("/api/index?__path=").data      # the page itself
    assert client.get("/api/index?__path=static/app.js").status_code == 200
    # query strings survive, the helper parameter does not leak into them, and the gate still applies to the restored path
    seen = {}
    web.fake.find = lambda q, **kw: seen.update(q=q) or []
    app2 = webapp.create_app()
    wrapped = Client(RestoreOriginalPath(app2.wsgi_app))
    assert wrapped.get("/api/index?__path=api/vendors").status_code == 401
    ok = wrapped.get("/api/index?__path=api/search&vendor=jlc&q=lm358", headers=web.h)
    assert ok.status_code == 200 and seen["q"] == "lm358"
    assert wrapped.get("/api/index?__path=api/../../etc/passwd", headers=web.h).status_code == 404
    assert Client(RestoreOriginalPath(app2.wsgi_app)).get("/api/vendors", headers=web.h).status_code == 200      # un-rewritten paths are untouched
