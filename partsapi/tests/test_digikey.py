import json

import pytest

from conftest import FakeResponse, FakeSession, fixture_json, make_http
from partsapi.digikey import DigiKey, _parse
from partsapi.errors import AuthError, NotFoundError, NoModelError

KW = fixture_json("digikey_keyword_synthetic.json")
TOKEN = {"access_token": "tok-1", "expires_in": 599, "token_type": "Bearer"}
SEARCH = "/search/keyword"


def client(session, kw=None, **over):
    session.add("POST", "/v1/oauth2/token", FakeResponse(200, json_data=TOKEN))
    session.add("POST", SEARCH, FakeResponse(200, json_data=kw or KW))
    session.add("GET", "/search/manufacturers", FakeResponse(200, json_data=fixture_json("digikey_manufacturers_synthetic.json")))
    session.add("GET", "/search/categories", FakeResponse(200, json_data=fixture_json("digikey_categories_synthetic.json")))
    return DigiKey(make_http(session), over.pop("client_id", "cid"), over.pop("client_secret", "sec"), **over)


def searches(session):
    return [c for c in session.calls if c["url"].endswith(SEARCH)]


# ------------------------------------------------------------------ auth / transport
def test_missing_credentials(session, monkeypatch):
    monkeypatch.delenv("DIGIKEY_CLIENT_ID", raising=False)
    monkeypatch.delenv("DIGIKEY_CLIENT_SECRET", raising=False)
    c = DigiKey(make_http(session))
    assert not c.is_configured()
    with pytest.raises(AuthError, match="DIGIKEY_CLIENT_ID"):
        c.findpart("lm358")
    assert session.calls == []


def test_env_credentials_and_locale(session, monkeypatch):
    monkeypatch.setenv("DIGIKEY_CLIENT_ID", "env-id")
    monkeypatch.setenv("DIGIKEY_CLIENT_SECRET", "env-secret")
    monkeypatch.setenv("DIGIKEY_SITE", "DE")
    monkeypatch.setenv("DIGIKEY_CURRENCY", "EUR")
    monkeypatch.setenv("DIGIKEY_SANDBOX", "1")
    c = client(session, client_id=None, client_secret=None)
    c = DigiKey(c.http)
    c.findpart("lm358")
    tok, call = session.calls[0], searches(session)[0]
    assert tok["url"].startswith("https://sandbox-api.digikey.com/v1/oauth2/token")
    assert tok["data"] == {"client_id": "env-id", "client_secret": "env-secret", "grant_type": "client_credentials"}
    h = call["headers"]
    assert h["Authorization"] == "Bearer tok-1" and h["X-DIGIKEY-Client-Id"] == "env-id"
    assert h["X-DIGIKEY-Locale-Site"] == "DE" and h["X-DIGIKEY-Locale-Currency"] == "EUR" and h["X-DIGIKEY-Locale-Language"] == "en"


def test_token_is_reused(session):
    c = client(session)
    c.findpart("lm358")
    c.findpart("lm358 mfr:yageo")
    assert sum("oauth2/token" in x["url"] for x in session.calls) == 1


def test_expired_token_is_refreshed_once(session):
    answers = iter([FakeResponse(401, text="expired"), FakeResponse(200, json_data=KW)])
    session.add("POST", "/v1/oauth2/token", FakeResponse(200, json_data=TOKEN))
    session.add("POST", SEARCH, lambda **kw: next(answers))
    c = DigiKey(make_http(session), "cid", "sec")
    assert c.findpart("lm358")
    assert sum("oauth2/token" in x["url"] for x in session.calls) == 2


def test_rejected_credentials(session):
    session.add("POST", "/v1/oauth2/token", FakeResponse(401, text='{"ErrorMessage":"Invalid clientId"}'))
    with pytest.raises(AuthError, match="rejected"):
        DigiKey(make_http(session), "bad", "bad").findpart("lm358")
    s2 = FakeSession().add("POST", "/v1/oauth2/token", FakeResponse(200, json_data=TOKEN)).add("POST", SEARCH, FakeResponse(403, text="no subscription"))
    with pytest.raises(AuthError, match="Product Information V4"):
        DigiKey(make_http(s2), "a", "b").findpart("lm358")
    s3 = FakeSession().add("POST", "/v1/oauth2/token", FakeResponse(200, json_data={"error": "x"}))
    with pytest.raises(AuthError, match="no access_token"):
        DigiKey(make_http(s3), "a", "b").findpart("lm358")


# ------------------------------------------------------------------ parsing
def test_parse_resistor_product():
    p = _parse(KW["Products"][1], "USD")
    assert (p.vendor, p.code, p.mpn, p.manufacturer) == ("digikey", "311-10.0KHRCT-ND", "RC0603FR-0710KL", "YAGEO")
    assert p.package == "0603 (1608 Metric)" and p.category == "Resistors / Chip Resistor - Surface Mount"
    assert p.attributes["Resistance"] == "10 kOhms" and p.extra["rohs"] is True and p.extra["status"] == "Active"
    assert [(b.min_qty, b.max_qty, b.unit_price) for b in p.price_breaks] == [(1, 9, 0.1), (10, 99, 0.034), (100, 999, 0.0146), (1000, None, 0.0089)]
    assert p.stock.quantity == 1800000 and p.stock.in_stock and len(p.stock.detail["variations"]) == 2
    assert p.extra["min_order_qty"] == 1 and p.extra["variations"][1]["min_order_qty"] == 5000


def test_parse_ic_normalises_urls_and_picks_variation():
    p = _parse(KW["Products"][0], "USD")
    assert p.datasheet_url == "https://www.ti.com/lit/ds/symlink/lm358.pdf"
    assert p.extra["eccn"] == "EAR99" and p.extra["htsus"] == "8542.33.0001" and p.extra["series"] == "LM358"
    assert p.stock.delivery == "manufacturer lead time 6 weeks"
    reel = _parse(KW["Products"][0], "USD", code="296-1395-2-ND")
    assert reel.code == "296-1395-2-ND" and reel.price_breaks[0].min_qty == 2500 and reel.extra["package_type"] == "Tape & Reel (TR)"
    assert _parse(KW["Products"][2], "USD").extra["rohs"] is False       # "Non-RoHS Compliant"
    assert _parse(KW["Products"][2], "USD").stock.in_stock is False


# ------------------------------------------------------------------ search / pushdown
def test_findpart_basic_and_body(session):
    c = client(session)
    out = c.findpart("lm358", limit=10, strict=False)       # digit words are otherwise required in the part text
    assert {p.mpn for p in out} == {"LM358DR", "RC0603FR-0710KL", "RC0603FR-07100KL", "ERJ-3EKF1002V"}
    body = searches(session)[0]["json"]
    assert body["Keywords"] == "lm358" and body["Limit"] == 10 and body["Offset"] == 0
    assert "FilterOptionsRequest" not in body and "SortOptions" not in body
    assert [p.mpn for p in c.findpart("lm358", limit=10)] == ["LM358DR"]          # strict: only text that contains it


def test_filters_are_pushed_to_digikey(session):
    c = client(session)
    c.findpart("resistor is:instock is:rohs has:datasheet has:image has:3dmodel stock:>=1000 mfr:=YAGEO cat:resistors")
    fo = searches(session)[0]["json"]["FilterOptionsRequest"]
    assert fo["SearchOptions"] == ["InStock", "RohsCompliant", "HasDatasheet", "HasProductPhoto", "Has3DModel"]
    assert fo["MinimumQuantityAvailable"] == 1000
    assert fo["ManufacturerFilter"] == [{"Id": "603"}]
    assert fo["CategoryFilter"] == [{"Id": "52"}]                    # exact name beats substring ("...Resistor - Surface Mount")
    client(session).findpart("resistor cat:chip")                    # no exact name: substring fallback
    assert searches(session)[-1]["json"]["FilterOptionsRequest"]["CategoryFilter"] == [{"Id": "53"}]


def test_manufacturer_substring_resolves_to_all_matching_ids(session):
    client(session).findpart("resistor mfr:panasonic")
    assert searches(session)[0]["json"]["FilterOptionsRequest"]["ManufacturerFilter"] == [{"Id": "10"}, {"Id": "11"}]


def test_unknown_manufacturer_or_category_returns_empty_without_searching(session):
    c = client(session)
    assert c.findpart("resistor mfr:nonexistentcorp") == []
    assert c.findpart("resistor cat:nonexistentcategory") == []
    assert searches(session) == []


def test_negated_conditions_stay_client_side(session):
    client(session).findpart("resistor -mfr:yageo")
    assert "ManufacturerFilter" not in searches(session)[0]["json"].get("FilterOptionsRequest", {})


def test_client_side_filters_over_parameters(session):
    c = client(session)
    got = c.findpart("resistor res:>=10k tol:<=1% pkg:0603 is:instock", limit=10)
    assert [p.mpn for p in got] == ["RC0603FR-0710KL", "ERJ-3EKF1002V"]     # 100k is out of stock, 10k and 1M pass
    assert all(p.stock.in_stock for p in got)
    assert [p.mpn for p in c.findpart("resistor res:100k")] == ["RC0603FR-07100KL"]
    assert [p.mpn for p in c.findpart("resistor power:>=0.1W volt:>=1V")] == []
    # price at quantity: YAGEO 10k at qty 1000 is 0.0089
    assert [p.mpn for p in c.findpart("RC0603FR price:<0.01 mfr:yageo", qty=1000)] == ["RC0603FR-0710KL"]


def test_sorting_pushdown_and_local_sort(session):
    c = client(session)
    out = c.findpart("resistor", sort="price", reverse=False)
    assert searches(session)[0]["json"]["SortOptions"] == {"Field": "Price", "SortOrder": "Ascending"}
    prices = [p.price_for(1) for p in out]
    assert prices == sorted(prices)
    c.findpart("resistor", sort="stock", reverse=True)
    assert searches(session)[-1]["json"]["SortOptions"] == {"Field": "QuantityAvailable", "SortOrder": "Descending"}
    c.findpart("resistor", sort="price", qty=100)                  # server sorts by unit price only
    assert "SortOptions" not in searches(session)[-1]["json"]


def test_paging_uses_offsets_and_stops(session):
    page1 = {"Products": [KW["Products"][1]] * 50, "ProductsCount": 120, "SearchLocaleUsed": {"Currency": "USD"}}
    page2 = {"Products": [KW["Products"][0]] * 50, "ProductsCount": 120}
    page3 = {"Products": [], "ProductsCount": 120}
    pages = iter([page1, page2, page3])
    session.add("POST", "/v1/oauth2/token", FakeResponse(200, json_data=TOKEN))
    session.add("POST", SEARCH, lambda **kw: FakeResponse(200, json_data=next(pages)))
    c = DigiKey(make_http(session), "a", "b")
    out = c.findpart("anything mpn:nomatch", limit=5, max_pages=5)
    assert out == []
    offs = [x["json"]["Offset"] for x in searches(session)]
    assert offs == [0, 50, 100] and all(x["json"]["Limit"] == 50 for x in searches(session))


def test_validation(session):
    c = client(session)
    with pytest.raises(ValueError, match="search word"):
        c.findpart("mfr:yageo")
    with pytest.raises(ValueError, match="lib"):
        c.findpart("resistor lib:basic")
    with pytest.raises(ValueError, match="preferred"):
        c.findpart("resistor is:preferred")
    with pytest.raises(ValueError):
        c.findpart("x", limit=0)


def test_extra_filter_options_merge(session):
    client(session).findpart("resistor", extra={"StatusFilter": [{"Id": "0"}]})
    assert searches(session)[0]["json"]["FilterOptionsRequest"]["StatusFilter"] == [{"Id": "0"}]


# ------------------------------------------------------------------ parametric pushdown
def parametric_response(total, values, pid=2085, name="Resistance"):
    r = json.loads(json.dumps(KW))
    r["ProductsCount"] = total
    r["FilterOptions"]["ParametricFilters"] = [{"Category": {"Id": 53, "Value": "Chip Resistor", "ProductCount": total}, "ParameterType": "String",
                                                "ParameterId": pid, "ParameterName": name,
                                                "FilterValues": [{"ProductCount": c, "ValueId": vid, "ValueName": vn} for vid, vn, c in values]}]
    return r


VALUES = [("v1", "1 kOhms", 10), ("v10", "10 kOhms", 50), ("v47", "47 kOhms", 20), ("v1m", "1 MOhms", 20)]


def test_complete_facets_become_parameter_filter(session):
    probe = parametric_response(100, VALUES)
    answers = iter([probe, KW])
    session.add("POST", "/v1/oauth2/token", FakeResponse(200, json_data=TOKEN))
    session.add("POST", SEARCH, lambda **kw: FakeResponse(200, json_data=next(answers)))
    session.add("GET", "/search/categories", FakeResponse(200, json_data=fixture_json("digikey_categories_synthetic.json")))
    c = DigiKey(make_http(session), "a", "b")
    c.findpart("resistor cat:=\"Chip Resistor - Surface Mount\" res:>=10k")
    probe_call, real = searches(session)
    assert probe_call["json"]["Limit"] == 1 and "ParameterFilterRequest" not in probe_call["json"]["FilterOptionsRequest"]
    assert real["json"]["FilterOptionsRequest"]["ParameterFilterRequest"] == {
        "CategoryFilter": {"Id": "53"},
        "ParameterFilters": [{"ParameterId": 2085, "FilterValues": [{"Id": "v10"}, {"Id": "v47"}, {"Id": "v1m"}]}]}


def test_facet_counts_below_total_still_push(session):
    # real DigiKey facets never add up to the total (products lacking the parameter): that must not block pushdown
    probe = parametric_response(500, VALUES)
    answers = iter([probe, KW])
    session.add("POST", "/v1/oauth2/token", FakeResponse(200, json_data=TOKEN))
    session.add("POST", SEARCH, lambda **kw: FakeResponse(200, json_data=next(answers)))
    session.add("GET", "/search/categories", FakeResponse(200, json_data=fixture_json("digikey_categories_synthetic.json")))
    DigiKey(make_http(session), "a", "b").findpart("resistor cat:=\"Chip Resistor - Surface Mount\" res:>=10k")
    assert "ParameterFilterRequest" in searches(session)[1]["json"]["FilterOptionsRequest"]


def test_category_without_parametric_facets_stays_client_side(session):
    # real behaviour: top-level categories return no ParametricFilters
    top = json.loads(json.dumps(KW)); top["FilterOptions"]["ParametricFilters"] = []
    answers = iter([top, KW])
    session.add("POST", "/v1/oauth2/token", FakeResponse(200, json_data=TOKEN))
    session.add("POST", SEARCH, lambda **kw: FakeResponse(200, json_data=next(answers)))
    session.add("GET", "/search/categories", FakeResponse(200, json_data=fixture_json("digikey_categories_synthetic.json")))
    DigiKey(make_http(session), "a", "b").findpart("resistor cat:=Resistors res:>=10k")
    assert "ParameterFilterRequest" not in searches(session)[1]["json"]["FilterOptionsRequest"]


def test_impossible_parameter_value_returns_empty(session):
    answers = iter([parametric_response(100, VALUES)])
    session.add("POST", "/v1/oauth2/token", FakeResponse(200, json_data=TOKEN))
    session.add("POST", SEARCH, lambda **kw: FakeResponse(200, json_data=next(answers)))
    session.add("GET", "/search/categories", FakeResponse(200, json_data=fixture_json("digikey_categories_synthetic.json")))
    assert DigiKey(make_http(session), "a", "b").findpart("resistor cat:=\"Chip Resistor - Surface Mount\" res:>1G") == []
    assert len(searches(session)) == 1


def test_no_parametric_push_without_single_category(session):
    client(session).findpart("resistor res:>=10k")
    assert len(searches(session)) == 1                              # no probe request


# ------------------------------------------------------------------ single part
def details_client(session):
    session.add("GET", "/productdetails", FakeResponse(200, json_data=fixture_json("digikey_productdetails_synthetic.json")))
    session.add("POST", "/v1/oauth2/token", FakeResponse(200, json_data=TOKEN))
    session.add("GET", "/search/manufacturers", FakeResponse(200, json_data=fixture_json("digikey_manufacturers_synthetic.json")))
    return DigiKey(make_http(session), "a", "b")


def test_getpart_by_digikey_number_picks_variation(session):
    c = details_client(session)
    p = c.getpart("296-1395-2-ND")
    assert p.code == "296-1395-2-ND" and p.price_breaks[0].min_qty == 2500
    assert session.calls[-1]["url"].endswith("/products/v4/search/296-1395-2-ND/productdetails")
    c.getpart("a/b c")
    assert session.calls[-1]["url"].endswith("/search/a%2Fb%20c/productdetails")


def test_getpart_manufacturer_disambiguation(session):
    c = details_client(session)
    c.getpart("LM358DR", manufacturer="Texas Instruments")
    assert session.calls[-1]["params"] == {"manufacturerId": 296}
    with pytest.raises(NotFoundError, match="manufacturer"):
        c.getpart("LM358DR", manufacturer="Nobody Inc")


def test_getpart_falls_back_to_keyword_exact_match(session):
    session.add("GET", "/productdetails", FakeResponse(404, text="not found"))
    c = client(session)
    p = c.getpart("rc0603fr-0710kl")
    assert p.mpn == "RC0603FR-0710KL"
    with pytest.raises(NotFoundError):
        c.getpart("no-such-part")
    with pytest.raises(ValueError):
        c.getpart("  ")


def test_price_stock_attributes_assets(session, tmp_path):
    c = details_client(session)
    session.add("GET", "mm.digikey.com", FakeResponse(200, content=b"JPG"))
    session.add("GET", "ti.com/lit", FakeResponse(200, content=b"%PDF"))
    assert c.getprice("296-1395-1-ND", 100) == 0.2
    assert c.getprice("296-1395-1-ND", 1) == 0.42
    assert [b.min_qty for b in c.getpartprice("296-1395-1-ND")] == [1, 10, 100]
    # an MPN spans package types: the cheapest orderable at that quantity wins (reel MOQ 2500)
    assert c.getprice("LM358DR", 100) == 0.2
    assert c.getprice("LM358DR", 5000) == 0.11
    assert c.getstock("LM358DR").quantity == 120000
    assert c.getattributes("LM358DR")["Number of Circuits"] == "2"
    assert c.getpic("LM358DR", tmp_path).name == "296-1395-1-ND.jpg"
    assert c.getdatasheet("LM358DR", tmp_path).read_bytes() == b"%PDF"


def test_get3dmodel_uses_exact_mpn(session):
    c = details_client(session)
    ee = {"success": True, "result": {"lists": {"lcsc": [{"uuid": "u", "dataStr": {"head": {"c_para": {"Supplier Part": "C7950", "Manufacturer Part": "LM358DR2G"}}}},
                                                          {"uuid": "v", "dataStr": {"head": {"c_para": {"Supplier Part": "C9999", "Manufacturer Part": "LM358DR"}}}}]}}}
    session.add("POST", "components/search", FakeResponse(200, json_data=ee))
    session.add("GET", "/api/products/C9999/components", FakeResponse(200, json_data=fixture_json("easyeda_component_C2913204.json")))
    session.add("GET", "qAxj6KHrDKw4blvCG8QJPs7Y/", FakeResponse(200, content=b"ISO-10303-21;"))
    assert c.get3dmodel("LM358DR").source_code == "C9999"
    session.add("POST", "components/search", FakeResponse(200, json_data={"success": True, "result": {"lists": {"lcsc": []}}}))
    # first matching route wins in the fake; a fresh client proves the no-match path
    s2 = FakeSession().add("GET", "/productdetails", FakeResponse(200, json_data=fixture_json("digikey_productdetails_synthetic.json")))
    s2.add("POST", "/v1/oauth2/token", FakeResponse(200, json_data=TOKEN)).add("POST", "components/search", FakeResponse(200, json_data={"success": True, "result": {"lists": {"lcsc": []}}}))
    with pytest.raises(NoModelError):
        DigiKey(make_http(s2), "a", "b").get3dmodel("LM358DR")


def test_facets(session):
    f = client(session).facets("resistor cat:resistors")
    assert f["total"] == 4 and f["manufacturers"][0] == ("Texas Instruments", 1) and f["packaging"] == [("Cut Tape (CT)", 4)]
    assert f["categories"] == [("Chip Resistor - Surface Mount", 3)]
    with pytest.raises(ValueError):
        client(FakeSession()).facets("mfr:ti")


def test_parameter_filter_does_not_leak_between_or_alternatives(session):
    # first alternative resolves a parametric filter; the second one has no facets => must not inherit it
    with_facets, without = parametric_response(100, VALUES), json.loads(json.dumps(KW))
    answers = iter([with_facets, KW, without, KW])
    session.add("POST", "/v1/oauth2/token", FakeResponse(200, json_data=TOKEN))
    session.add("POST", SEARCH, lambda **kw: FakeResponse(200, json_data=next(answers)))
    session.add("GET", "/search/categories", FakeResponse(200, json_data=fixture_json("digikey_categories_synthetic.json")))
    DigiKey(make_http(session), "a", "b").findpart('chip OR resistor cat:="Chip Resistor - Surface Mount" res:>=10k', strict=False)
    real = [c for c in searches(session) if c["json"]["Limit"] != 1]
    assert "ParameterFilterRequest" in real[0]["json"]["FilterOptionsRequest"]
    assert "ParameterFilterRequest" not in real[1]["json"]["FilterOptionsRequest"]


def test_short_dk_environment_names_work_and_long_names_win(session, monkeypatch):
    for k in ("DIGIKEY_CLIENT_ID", "DIGIKEY_CLIENT_SECRET", "DIGIKEY_SANDBOX"):
        monkeypatch.delenv(k, raising=False)
    monkeypatch.setenv("DK_CLIENT_ID", "short-id")
    monkeypatch.setenv("DK_CLIENT_SECRET", "short-secret")
    monkeypatch.setenv("DK_SANDBOX", "1")
    c = DigiKey(make_http(session))
    assert c.is_configured() and (c.client_id, c.client_secret) == ("short-id", "short-secret")
    assert c.sandbox and c.base == "https://sandbox-api.digikey.com"
    monkeypatch.setenv("DIGIKEY_CLIENT_ID", "long-id")
    assert DigiKey(make_http(session)).client_id == "long-id"


def test_default_variation_is_the_one_orderable_in_the_smallest_quantity():
    prod = json.loads(json.dumps(KW["Products"][0]))
    ct, reel, digireel = prod["ProductVariations"]
    prod["ProductVariations"] = [reel, ct, digireel]          # reel (MOQ 2500) first, as the live API returns it
    p = _parse(prod, "USD")
    assert p.code == "296-1395-1-ND" and p.extra["min_order_qty"] == 1 and p.price_breaks[0].min_qty == 1
    assert _parse(prod, "USD", code="296-1395-2-ND").code == "296-1395-2-ND"     # explicit number still wins


def test_package_filter_is_pushed_as_the_package_case_parameter(session):
    values = [("a", "0201 (0603 Metric)", 5), ("b", "0603 (1608 Metric)", 9), ("c", "0805 (2012 Metric)", 7)]
    answers = iter([parametric_response(21, values, pid=16, name="Package / Case"), KW])
    session.add("POST", "/v1/oauth2/token", FakeResponse(200, json_data=TOKEN))
    session.add("POST", SEARCH, lambda **kw: FakeResponse(200, json_data=next(answers)))
    session.add("GET", "/search/categories", FakeResponse(200, json_data=fixture_json("digikey_categories_synthetic.json")))
    DigiKey(make_http(session), "a", "b").findpart('resistor cat:="Chip Resistor - Surface Mount" pkg:0603')
    sent = searches(session)[1]["json"]["FilterOptionsRequest"]["ParameterFilterRequest"]
    assert sent["ParameterFilters"] == [{"ParameterId": 16, "FilterValues": [{"Id": "b"}]}]   # not the 0201 (0603 Metric) value
