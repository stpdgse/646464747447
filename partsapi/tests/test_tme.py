import base64

import pytest

from conftest import FakeResponse, FakeSession, fixture_json, make_http
from partsapi.errors import AuthError, NoModelError, NotFoundError
from partsapi.tme import TME, _apply_data, _apply_params, _card

TOKEN = fixture_json("tme_token.json")
SEARCH = fixture_json("tme_search_lm358.json")
DATA = fixture_json("tme_data.json")
FILES = fixture_json("tme_files.json")
PARAMS = fixture_json("tme_parameters.json")
PRODUCT = fixture_json("tme_product.json")
FACETS = fixture_json("tme_facets_resistor.json")
CATS = fixture_json("tme_categories_small.json")
MFRS = fixture_json("tme_manufacturers.json")
CARDS = SEARCH["data"]["products"]["elements"]


def pv(call, key):
    """All values of a (repeatable) query parameter of a recorded call."""
    return [v for k, v in call["params"] if k == key]


def paths(session, p):
    return [c for c in session.calls if c["url"].endswith(p)]


def ok(data):
    return FakeResponse(200, json_data={"status": "OK", "data": data})


def make(session, tmp_path, search=None, **kw):
    """A TME client whose fake API answers every endpoint from the real fixtures."""
    session.add("POST", "/auth/token", FakeResponse(200, json_data=TOKEN))
    session.add("GET", "/products/search", lambda **k: search(**k) if search else FakeResponse(200, json_data=SEARCH))
    session.add("GET", "/products/data", lambda **k: ok({"elements": [e for e in DATA["data"]["elements"] if e["symbol"] in pv(k, "symbols[]")]}))
    session.add("GET", "/products/parameters", lambda **k: ok({"elements": [e for e in PARAMS["data"]["elements"] if e["symbol"] in pv(k, "symbols[]")]}))
    session.add("GET", "/products/files", lambda **k: ok({"elements": [e for e in FILES["data"]["elements"] if e["symbol"] in pv(k, "symbols[]")]}))
    session.add("GET", "/products/categories/list", FakeResponse(200, json_data=CATS))
    session.add("GET", "/products/manufacturers", FakeResponse(200, json_data=MFRS))
    session.add("GET", "api.tme.eu/products", lambda **k: ok({"elements": [e for e in PRODUCT["data"]["elements"] if e["symbol"] in pv(k, "symbols[]") or e["manufacturer_symbols"][0] in pv(k, "mpns[]")]}))
    return TME(make_http(session), "the-token", "the-secret", cache_dir=tmp_path, **kw)


# ------------------------------------------------------------------ auth
def test_missing_credentials(session, tmp_path):
    t = TME(make_http(session), cache_dir=tmp_path)
    assert not t.is_configured()
    with pytest.raises(AuthError, match="TME_TOKEN"):
        t.findpart("lm358")
    assert session.calls == []


def test_env_credentials_and_country(session, tmp_path, monkeypatch):
    monkeypatch.setenv("TME_TOKEN", "env-token")
    monkeypatch.setenv("TME_SECRET", "env-secret")
    monkeypatch.setenv("TME_COUNTRY", "PL")
    t = TME(make_http(session), cache_dir=tmp_path)
    assert (t.token, t.secret, t.country, t.currency) == ("env-token", "env-secret", "PL", None)
    monkeypatch.delenv("TME_COUNTRY")
    d = TME(make_http(session), "a", "b", cache_dir=tmp_path)
    assert (d.country, d.currency) == ("DE", "EUR")


def test_token_request_and_bearer_header(session, tmp_path):
    t = make(session, tmp_path)
    t.getstock("LM358DT")
    tok = paths(session, "/auth/token")[0]
    assert tok["data"] == {"grant_type": "client_credentials"}
    assert tok["headers"]["Authorization"] == "Basic " + base64.b64encode(b"the-token:the-secret").decode()
    call = paths(session, "/products/data")[0]
    assert call["headers"]["Authorization"] == "Bearer test-access-token"
    assert ("country", "DE") in call["params"] and ("currency", "EUR") in call["params"]
    t.getstock("LM358DT")
    assert len(paths(session, "/auth/token")) == 1                                  # token reused


def test_expired_access_token_is_refreshed_once(session, tmp_path):
    answers = iter([FakeResponse(401, text="expired"), ok(DATA["data"])])
    t = make(session, tmp_path)
    session.routes.insert(0, ("GET", "/products/data", lambda **k: next(answers)))
    assert t.getstock("LM358DT").quantity == 2250
    assert len(paths(session, "/auth/token")) == 2


def test_rejected_credentials(session, tmp_path):
    session.add("POST", "/auth/token", FakeResponse(400, text='{"error":"invalid_request","error_description":"E_TOKEN_NOT_FOUND"}'))
    with pytest.raises(AuthError, match="14 May 2026"):
        TME(make_http(session), "bad", "bad", cache_dir=tmp_path).findpart("lm358")
    s2 = FakeSession().add("POST", "/auth/token", FakeResponse(401, text="no"))
    with pytest.raises(AuthError):
        TME(make_http(s2), "bad", "bad", cache_dir=tmp_path).findpart("lm358")
    s3 = FakeSession().add("POST", "/auth/token", FakeResponse(200, json_data={"nothing": 1}))
    with pytest.raises(AuthError, match="no access_token"):
        TME(make_http(s3), "a", "b", cache_dir=tmp_path).findpart("lm358")
    s4 = FakeSession().add("POST", "/auth/token", FakeResponse(200, json_data=TOKEN)).add("GET", "/products/data", FakeResponse(403, text="no"))
    with pytest.raises(AuthError, match="refused"):
        TME(make_http(s4), "a", "b", cache_dir=tmp_path).getstock("X")


# ------------------------------------------------------------------ parsing real responses
def test_card_from_real_search_hit():
    p = _card(CARDS[1])
    assert (p.vendor, p.code, p.mpn, p.manufacturer) == ("tme", "LM358DT", "LM358DT", "STMicroelectronics")
    assert p.description.startswith("IC: operational amplifier") and p.category == "SMD operational amplifiers"
    assert p.image_urls == [] or all(u.startswith("https://") for u in p.image_urls)
    assert p.extra["packing"] == [{"type": "RL1", "quantity": 2500}] and p.extra["min_order_qty"] == 1 and p.extra["category_id"] == 112843
    first = _card(CARDS[0])
    assert first.mpn == "MIKROE-4357" and first.extra["manufacturer_symbols"] == ["MIKROE-4357", "BRUSHLESS 12 CLICK"]
    assert first.image_urls[0].startswith("https://ce8dc832c.cloudimg.io/") and "width=1440" in first.image_urls[0]     # high resolution first


def test_data_applies_ladder_vat_and_stock():
    p = _card(CARDS[1])
    _apply_data(p, DATA["data"]["elements"][0])
    assert [(b.min_qty, b.max_qty, b.unit_price) for b in p.price_breaks][:3] == [(1, 4, 0.13), (5, 9, 0.112), (10, 24, 0.102)]
    assert p.price_breaks[-1].max_qty is None and p.price_breaks[0].currency == "EUR"
    assert p.price_breaks[0].incl_vat is True and p.price_breaks[0].vat_rate == pytest.approx(0.19)
    assert p.stock.quantity == 2250 and p.stock.in_stock and p.extra["price_type"] == "GROSS"


def test_net_prices_and_missing_stock():
    p = _card(CARDS[1])
    net = {"stock_quantity": None, "symbol": "LM358DT", "prices": {"elements": [{"amount": 1, "price": 0.1, "special": True}], "tax": None, "currency": "USD", "type": "NET"}}
    _apply_data(p, net)
    assert p.price_breaks[0].incl_vat is False and p.price_breaks[0].vat_rate is None and p.price_breaks[0].currency == "USD"
    assert p.stock.quantity is None and not p.stock.in_stock and p.extra["special_prices"] == [1]


def test_parameters_become_attributes_and_package():
    p = _card(CARDS[1])
    _apply_params(p, PARAMS["data"]["elements"][0])
    assert p.attributes["Bandwidth"] == "1.1MHz" and p.attributes["Kind of package"] == "reel, tape"
    assert p.package == "SO8"
    r = _card(CARDS[1])
    _apply_params(r, {"parameters": {"elements": [{"name": "Case - inch", "values": [{"value": "0603"}]}, {"name": "Case - mm", "values": [{"value": "1608"}]}]}})
    assert r.package == "0603" and r.extra["case_mm"] == "1608"


# ------------------------------------------------------------------ search
def test_search_request_shape_and_hydration(session, tmp_path):
    t = make(session, tmp_path)
    out = t.findpart("lm358", limit=10)
    s = paths(session, "/products/search")[0]
    assert ("phrase", "lm358") in s["params"] and ("scope[]", "products") in s["params"] and ("scope[]", "counters") in s["params"]
    assert ("limit", 100) in s["params"] and ("page", 1) in s["params"] and ("country", "DE") in s["params"]
    assert [p.code for p in out] == ["MIKROE-4357", "LM358DT"]       # the click board's description lists an LM358 too
    board, p = out
    assert board.price_breaks == [] and board.stock is None           # (the fixture has no price data for it)
    assert p.price_for(1) == 0.13 and p.stock.quantity == 2250 and p.attributes["Case"] == "SO8" and p.package == "SO8"


def test_strict_and_text_are_checked_after_parameters_are_known(session, tmp_path):
    t = make(session, tmp_path)
    assert {p.code for p in t.findpart("lm358", strict=False, limit=10)} == {"LM358DT", "MIKROE-4357"}
    assert t.findpart("lm358 0805") == []                                                        # digit word not in text or parameters
    assert [p.code for p in t.findpart("lm358 so8")] == ["LM358DT"]                              # "SO8" is only in its Case parameter


def test_server_side_filters_sort_and_pages(session, tmp_path):
    t = make(session, tmp_path)
    t.findpart('lm358 is:instock cat:="SMD operational amplifiers" mfr:=STMicroelectronics', sort="price", reverse=True, limit=5)
    s = paths(session, "/products/search")[0]["params"]
    assert ("category_id", 112843) in s and ("manufacturer_id", 35) in s and ("filter[in_stock]", "true") in s
    assert ("sort[property]", "PRICE_FIRST_QUANTITY") in s and ("sort[direction]", "desc") in s
    t.findpart("lm358", sort="mpn")
    assert ("sort[property]", "ORIGINAL_SYMBOL") in paths(session, "/products/search")[-1]["params"]
    t.findpart("lm358", sort="price", qty=100)                         # TME sorts by the single-unit price only
    assert not any(k == "sort[property]" for k, _ in paths(session, "/products/search")[-1]["params"])


def test_category_only_search_and_errors(session, tmp_path):
    t = make(session, tmp_path)
    t.findpart('cat:="SMD operational amplifiers"')
    s = paths(session, "/products/search")[-1]["params"]
    assert ("category_id", 112843) in s and not any(k == "phrase" for k, _ in s)
    with pytest.raises(ValueError, match="similar: .*capacitors"):
        t.findpart("x1 cat:capcitors")                                 # a typo still finds the real names
    with pytest.raises(ValueError, match="no TME category") as e:
        t.findpart('cat:="Ceramic caps" lm358')
    assert "Ceramic capacitors" in str(e.value)
    with pytest.raises(ValueError, match="at least one search word"):
        t.findpart("is:instock")
    with pytest.raises(ValueError, match="2-40"):
        t.findpart("x" * 41)
    with pytest.raises(ValueError, match="2-40"):
        t.findpart("a")


def test_ambiguous_category_is_an_error_not_a_guess(session, tmp_path):
    cats = {"status": "OK", "data": {"elements": [{"parent_id": 1, "id": i, "products_count": 1, "name": f"Widget {i}"} for i in range(2, 10)]}}
    session.add("POST", "/auth/token", FakeResponse(200, json_data=TOKEN))
    session.add("GET", "/products/categories/list", FakeResponse(200, json_data=cats))
    with pytest.raises(ValueError, match="matches 8 TME categories"):
        TME(make_http(session), "a", "b", cache_dir=tmp_path).findpart("x1 cat:widget")


def test_unknown_manufacturer_returns_empty(session, tmp_path):
    t = make(session, tmp_path)
    assert t.findpart("lm358 mfr:nonexistentcorp") == []
    assert not paths(session, "/products/search")


def test_multiple_manufacturer_matches_stay_client_side(session, tmp_path):
    t = make(session, tmp_path)
    t.findpart("lm358 mfr:o")                                          # substring of several names: no manufacturer_id
    assert not any(k == "manufacturer_id" for k, _ in paths(session, "/products/search")[0]["params"])


def test_client_side_filters_on_real_data(session, tmp_path):
    t = make(session, tmp_path)
    assert [p.code for p in t.findpart('lm358 price:<0.2 stock:>=1000 pkg:so8 @"Bandwidth":>=1MHz')] == ["LM358DT"]
    assert t.findpart('lm358 @"Bandwidth":>=2MHz') == []                # 1.1MHz is below
    assert t.findpart("lm358 volt:>=3V") == []                        # "3...30V DC" is a range, not a number: never satisfies a comparison
    assert t.findpart("lm358 price:<0.05") == [] and t.findpart("lm358 stock:>=5000") == []
    assert [p.code for p in t.findpart("lm358 -operational")] == ["MIKROE-4357"]       # removes LM358DT; the click board never says "operational"
    assert t.findpart("lm358 price:<0.09", qty=100) and not t.findpart("lm358 price:<0.08", qty=100)


def test_has_datasheet_fetches_documents(session, tmp_path):
    t = make(session, tmp_path)
    out = t.findpart("lm358 has:datasheet")
    assert out and out[0].datasheet_url == "https://www.tme.eu/Document/cf29666b1e00eaf05f9b879378749b20/LM358D-st.pdf"
    assert paths(session, "/products/files")


def test_paging_and_batching(session, tmp_path):
    many = [dict(CARDS[1], symbol=f"SYM{i}", manufacturer_symbols=[f"SYM{i}"]) for i in range(120)]
    pages = {1: many[:100], 2: many[100:]}
    def search(**k):
        pg = dict(k["params"])["page"]
        return FakeResponse(200, json_data={"status": "OK", "data": {"products": {"elements": pages[pg]}, "counters": {"pages": 2, "count": 120, "page": pg}}})
    t = make(session, tmp_path, search=search)
    out = t.findpart("anything price:<0", max_pages=5)                   # price is only known after hydration: all 120 hits are completed
    assert out == []
    assert [dict(c["params"])["page"] for c in paths(session, "/products/search")] == [1, 2]
    assert [len(pv(c, "symbols[]")) for c in paths(session, "/products/data")] == [50, 50, 20]      # never more than 50 symbols per call
    assert [len(pv(c, "symbols[]")) for c in paths(session, "/products/parameters")] == [50, 50, 20]


def test_or_runs_separate_searches(session, tmp_path):
    make(session, tmp_path).findpart("lm358 OR lm324", strict=False, max_pages=1)
    assert [dict(c["params"])["phrase"] for c in paths(session, "/products/search")] == ["lm358", "lm324"]


def test_unsupported_filters(session, tmp_path):
    t = make(session, tmp_path)
    for q in ("lm358 is:rohs", "lm358 is:preferred", "lm358 lib:basic", "lm358 has:3dmodel"):
        with pytest.raises(ValueError):
            t.findpart(q)
    with pytest.raises(ValueError):
        t.findpart("x", limit=0)


# ------------------------------------------------------------------ parametric pushdown (real facet structure)
def facet_search(session, tmp_path, result=None):
    def search(**k):
        if ("scope[]", "parameters") in k["params"]:
            return FakeResponse(200, json_data=FACETS)
        return FakeResponse(200, json_data=result or {"status": "OK", "data": {"products": {"elements": []}, "counters": {"pages": 1}}})
    return make(session, tmp_path, search=search)


def real_search_calls(session):
    return [c for c in paths(session, "/products/search") if ("scope[]", "parameters") not in c["params"]]


def value_ids(name):
    f = next(f for f in FACETS["data"]["parameters"]["elements"] if f["name"] == name)
    return f["id"], {v["value"]: v["id"] for v in f["values"]}


def test_numeric_attributes_become_parameter_filters(session, tmp_path):
    t = facet_search(session, tmp_path)
    t.findpart("resistor res:>=10k tol:<=1%")
    sent = real_search_calls(session)[0]["params"]
    rid, rv = value_ids("Resistance")
    tid, tv = value_ids("Tolerance")
    got = {}
    for k, v in sent:
        if k.startswith("parameters["):
            got.setdefault(k.split("]")[0] + "]", []).append(v)
    assert got["parameters[0]"][0] == rid and set(got["parameters[0]"][1:]) == {rv["10kΩ"], rv["47kΩ"], rv["100kΩ"], rv["1MΩ"]}    # 4.7k and 10Ω excluded
    assert got["parameters[1]"][0] == tid and set(got["parameters[1]"][1:]) == {tv["±1%"], tv["±0.5%"]}


def test_ambiguous_names_prefer_the_exact_parameter(session, tmp_path):
    # `res` also matches "Min. insulation resistance" and "Coil resistance"; "Resistance" is the one meant
    t = facet_search(session, tmp_path)
    t.findpart("resistor res:10k")
    rid, _ = value_ids("Resistance")
    assert ("parameters[0][id]", rid) in real_search_calls(session)[0]["params"]


def test_package_uses_inch_sizes_never_metric(session, tmp_path):
    t = facet_search(session, tmp_path)
    t.findpart("resistor pkg:0603")
    sent = real_search_calls(session)[0]["params"]
    iid, iv = value_ids("Case - inch")
    assert ("parameters[0][id]", iid) in sent and ("parameters[0][values][]", iv["0603"]) in sent
    mid, mv = value_ids("Case - mm")
    assert ("parameters[0][id]", mid) not in sent                       # "Case - mm" 0603 is imperial 0201


def test_ic_package_uses_the_case_parameter(session, tmp_path):
    t = facet_search(session, tmp_path)
    t.findpart("resistor pkg:melf")                                     # a value that exists in the "Case" parameter
    cid, cv = value_ids("Case")
    sent = real_search_calls(session)[0]["params"]
    assert ("parameters[0][id]", cid) in sent and ("parameters[0][values][]", cv["MELF"]) in sent


def test_impossible_values_return_empty_without_searching(session, tmp_path):
    t = facet_search(session, tmp_path)
    assert t.findpart("resistor res:>1G") == []
    assert t.findpart("resistor pkg:9999") == []
    assert real_search_calls(session) == []


def test_negated_and_unknown_attributes_stay_client_side(session, tmp_path):
    t = facet_search(session, tmp_path)
    t.findpart('resistor -res:>=10k @"No Such Parameter":x')
    assert not any(k.startswith("parameters[") for k, _ in real_search_calls(session)[0]["params"])


# ------------------------------------------------------------------ single part
def test_getpart_everything(session, tmp_path):
    p = make(session, tmp_path).getpart("LM358DT")
    assert (p.code, p.mpn, p.manufacturer, p.package) == ("LM358DT", "LM358DT", "STMicroelectronics", "SO8")
    assert p.category == "Semiconductors / Integrated circuits / Analog and mixed integrated circuits / Operational amplifiers / SMD operational amplifiers"
    assert p.price_for(100) == 0.0833 and p.stock.quantity == 2250
    assert p.datasheet_url == "https://www.tme.eu/Document/cf29666b1e00eaf05f9b879378749b20/LM358D-st.pdf"
    assert [d["type"] for d in p.extra["documents"]] == ["DTE", "LNK"] and p.attributes["Number of channels"] == "2"


def test_getpart_falls_back_to_mpn_and_reports_unknown(session, tmp_path):
    t = make(session, tmp_path)
    t.getpart("LM358DT")
    assert ("symbols[]", "LM358DT") in paths(session, "/products")[0]["params"]
    session.calls.clear()
    session.routes.insert(0, ("GET", "/products", lambda **k: ok({"elements": PRODUCT["data"]["elements"] if pv(k, "mpns[]") else []})))
    assert t.getpart("LM358-MPN").code == "LM358DT"
    assert [k for k, _ in paths(session, "/products")[0]["params"] if k.endswith("[]")] == ["symbols[]"]
    assert [k for k, _ in paths(session, "/products")[1]["params"] if k.endswith("[]")] == ["mpns[]"]
    s2 = FakeSession()
    s2.add("POST", "/auth/token", FakeResponse(200, json_data=TOKEN)).add("GET", "/products", ok({"elements": []}))
    with pytest.raises(NotFoundError):
        TME(make_http(s2), "a", "b", cache_dir=tmp_path).getpart("NOPE")
    with pytest.raises(ValueError):
        t.getpart("  ")


def test_price_stock_and_unknown_symbol(session, tmp_path):
    t = make(session, tmp_path)
    assert t.getprice("LM358DT", 100) == 0.0833 and t.getprice("LM358DT", 1) == 0.13 and len(t.getpartprice("LM358DT")) == 10
    assert t.getstock("LM358DT").quantity == 2250
    with pytest.raises(NotFoundError):
        t.getprice("UNKNOWN-1")


def test_attributes_pics_datasheet_3d(session, tmp_path):
    t = make(session, tmp_path)
    assert t.getattributes("LM358DT")["Mounting"] == "SMT"
    session.add("GET", "cloudimg.io", FakeResponse(200, content=b"JPG"))
    session.add("GET", "tme.eu/Document", FakeResponse(200, content=b"%PDF"))
    assert t.getpic("LM358DT", tmp_path).name == "LM358DT.jpg"
    assert t.getdatasheet("LM358DT", tmp_path).read_bytes() == b"%PDF"
    ee = {"success": True, "result": {"lists": {"lcsc": [{"uuid": "u", "dataStr": {"head": {"c_para": {"Supplier Part": "C9418", "Manufacturer Part": "LM358DT"}}}}]}}}
    session.add("POST", "components/search", FakeResponse(200, json_data=ee))
    session.add("GET", "/api/products/C9418/components", FakeResponse(200, json_data=fixture_json("easyeda_component_C2913204.json")))
    session.add("GET", "qAxj6KHrDKw4blvCG8QJPs7Y/", FakeResponse(200, content=b"ISO-10303-21;"))
    assert t.get3dmodel("LM358DT").source_code == "C9418"
    s2 = FakeSession().add("POST", "components/search", FakeResponse(200, json_data={"success": True, "result": {"lists": {"lcsc": []}}}))
    t2 = make(s2, tmp_path)
    s2.routes.append(("POST", "components/search", lambda **k: FakeResponse(200, json_data={"success": True, "result": {"lists": {"lcsc": []}}})))
    with pytest.raises(NoModelError):
        t2.get3dmodel("LM358DT")


def test_missing_datasheet_and_picture(session, tmp_path):
    bare = {"elements": [dict(PRODUCT["data"]["elements"][0], assets={"primary_photo": {}})]}
    session.add("POST", "/auth/token", FakeResponse(200, json_data=TOKEN))
    session.add("GET", "/products/files", ok({"elements": [{"symbol": "LM358DT", "assets": {}, "documents": {"elements": []}}]}))
    session.add("GET", "/products/data", ok(DATA["data"]))
    session.add("GET", "/products/parameters", ok(PARAMS["data"]))
    session.add("GET", "/products/categories/list", FakeResponse(200, json_data=CATS))
    session.add("GET", "/products", ok(bare))
    t = TME(make_http(session), "a", "b", cache_dir=tmp_path)
    with pytest.raises(NotFoundError, match="datasheet"):
        t.getdatasheet("LM358DT")
    with pytest.raises(NotFoundError, match="picture"):
        t.getpic("LM358DT")


# ------------------------------------------------------------------ facets + category tree cache
def test_facets_from_the_server(session, tmp_path):
    t = facet_search(session, tmp_path)
    f = t.facets("resistor")
    assert f["total"] == 100 and f["categories"] == [("Analog and mixed integrated circuits", 100)]
    assert dict(f["attributes"]["Tolerance"])["±1%"] > 0 and "Manufacturer" not in f["attributes"]
    with pytest.raises(ValueError):
        t.facets("is:instock")


def test_category_tree_is_cached_on_disk(session, tmp_path):
    t = make(session, tmp_path)
    assert t.category_path(113537) == "Passives / Capacitors / MLCC capacitors / MLCC SMD capacitors"
    assert (tmp_path / "tme_categories_DE.json").exists()
    s2 = FakeSession().add("POST", "/auth/token", FakeResponse(200, json_data=TOKEN))
    assert TME(make_http(s2), "a", "b", cache_dir=tmp_path).category_path(113537).endswith("MLCC SMD capacitors")
    assert not paths(s2, "/products/categories/list")
