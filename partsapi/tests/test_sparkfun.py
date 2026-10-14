import json

import pytest

from conftest import FakeResponse, FakeSession, fixture_json, fixture_text, make_http
from partsapi.errors import NoModelError, NotFoundError, PartsApiError
from partsapi.sparkfun import SparkFun, _category_path, _parse

SEARCH = fixture_json("sparkfun_search.json")
ITEM = fixture_json("sparkfun_item_COM-18354.json")
AGGS = fixture_json("sparkfun_aggs.json")
FILTERED = fixture_json("sparkfun_filtered.json")
CATS = fixture_json("sparkfun_categories.json")
PAGE = fixture_text("sparkfun_page_COM-18354.html")


def gql(session, routes):
    """Route GraphQL POSTs by a substring of the query; first match wins."""
    def respond(**kw):
        q = kw["json"]["query"]
        for part, payload in routes:
            if part in q:
                return FakeResponse(200, json_data=payload)
        raise AssertionError(f"unrouted GraphQL query: {q[:160]}")
    session.add("POST", "/graphql", respond)
    return SparkFun(make_http(session))


def queries(session):
    return [c["json"]["query"] for c in session.calls if c["url"].endswith("/graphql")]


# ------------------------------------------------------------------ parsing
def test_parse_real_product():
    p = _parse(ITEM["data"]["products"]["items"][0])
    assert (p.vendor, p.code, p.mpn) == ("sparkfun", "COM-18354", "COM-18354")
    assert p.description == "SparkFun Qwiic LED Stick - APA102C"
    assert p.category == "Components / LEDs and Illumination / Addressable"       # marketing categories are skipped
    assert [(b.min_qty, b.max_qty, b.unit_price, b.currency) for b in p.price_breaks] == [
        (1, 9, 12.5, "USD"), (10, 24, 11.88, "USD"), (25, 99, 11.25, "USD"), (100, None, 10.63, "USD")]
    assert p.stock.in_stock and p.stock.quantity is None and p.stock.detail["stock_status"] == "IN_STOCK"
    assert p.url == "https://www.sparkfun.com/sparkfun-qwiic-led-stick-apa102c.html"
    assert p.image_urls and all(u.startswith("https://www.sparkfun.com/media/") for u in p.image_urls)
    assert len(p.image_urls) == len(set(p.image_urls)) and p.extra["min_order_qty"] == 1 and p.extra["country_of_origin"] == "US"


def test_category_path_skips_marketing_and_builds_chain():
    cats = [{"name": "Components", "url_path": "components", "level": 2}, {"name": "Buttons and Switches", "url_path": "components/buttons-and-switches", "level": 3},
            {"name": "Button", "url_path": "components/buttons-and-switches/button", "level": 4},
            {"name": "Qwiic", "url_path": "special-categories/qwiic", "level": 3}]
    assert _category_path(cats) == "Components / Buttons and Switches / Button"
    assert _category_path([{"name": "Qwiic", "url_path": "special-categories/qwiic", "level": 3}]) is None
    assert _category_path([]) is None


def test_parse_out_of_stock_without_tiers():
    it = dict(ITEM["data"]["products"]["items"][0], stock_status="OUT_OF_STOCK", price_tiers=[])
    p = _parse(it)
    assert p.stock.in_stock is False and [b.min_qty for b in p.price_breaks] == [1]


# ------------------------------------------------------------------ search
def test_search_request_shape_and_results(session):
    c = gql(session, [("products(", SEARCH)])
    out = c.findpart("qwiic led", limit=10)
    assert [p.code for p in out] == [it["sku"] for it in SEARCH["data"]["products"]["items"]]
    q = queries(session)[0]
    assert 'search:"qwiic led"' in q and "pageSize:100" in q and "currentPage:1" in q and "sort:{relevance:DESC}" in q
    assert session.calls[0]["headers"]["Store"] == "default"


def test_client_side_filters_and_sort(session):
    c = gql(session, [("products(", SEARCH)])
    assert c.findpart("qwiic led price:<6") and all(p.price_for(1) < 6 for p in c.findpart("qwiic led price:<6"))
    out = c.findpart("qwiic led", sort="price")
    assert [p.price_for(1) for p in out] == sorted(p.price_for(1) for p in out)
    assert [p.code for p in c.findpart("qwiic led -stick")] == [p.code for p in c.findpart("qwiic led") if "stick" not in p.description.lower()]
    assert c.findpart("qwiic led is:instock")
    assert c.findpart("qwiic led stock:>=1000") == []                          # SparkFun gives no counts: only "> 0" can pass
    assert c.findpart("qwiic led stock:>0")


def test_paging_stops_at_last_page_and_max_pages(session):
    page = json.loads(json.dumps(SEARCH)); page["data"]["products"]["page_info"] = {"total_pages": 5}
    c = gql(session, [("products(", page)])
    c.findpart("qwiic mpn:nomatch", max_pages=2)
    assert len([q for q in queries(session) if "currentPage" in q]) == 2
    one = json.loads(json.dumps(SEARCH)); one["data"]["products"]["page_info"] = {"total_pages": 1}
    s2 = FakeSession()
    gql(s2, [("products(", one)]).findpart("qwiic mpn:nomatch", max_pages=5)
    assert len(queries(s2)) == 1


def test_or_runs_separate_searches(session):
    one_page = json.loads(json.dumps(SEARCH)); one_page["data"]["products"]["page_info"] = {"total_pages": 1}
    c = gql(session, [("products(", one_page)])
    c.findpart("qwiic OR stemma", strict=False)
    searches = [q for q in queries(session) if "search:" in q]
    assert len(searches) == 2 and 'search:"qwiic"' in searches[0] and 'search:"stemma"' in searches[1]


def test_category_filter_resolves_to_uids(session):
    c = gql(session, [("categories(", CATS), ("products(", SEARCH)])
    c.findpart('sensor cat:"Environmental Sensors"')
    uids = [i["uid"] for i in CATS["data"]["categories"]["items"] if i["name"] == "Environmental Sensors"]
    assert f'filter:{{category_uid:{{in:["{uids[0]}"]}}}}' in queries(session)[-1]


def test_unknown_category_returns_empty_without_searching(session):
    c = gql(session, [("categories(", {"data": {"categories": {"items": []}}})])
    assert c.findpart("sensor cat:nonexistent") == []
    assert not any("products(" in q for q in queries(session))


def test_attribute_filter_becomes_server_side_facet_filter(session):
    c = gql(session, [("aggregations", AGGS), ("products(", FILTERED)])
    out = c.findpart('sensor @"Sensor Type":humidity', limit=5)
    assert out and [p.code for p in out] == [it["sku"] for it in FILTERED["data"]["products"]["items"]]
    humidity = next(o["value"] for a in AGGS["data"]["products"]["aggregations"] if a["attribute_code"] == "sf_sensors_environmental_type"
                    for o in a["options"] if o["label"] == "Humidity")
    final = [q for q in queries(session) if "aggregations" not in q][-1]
    assert f'filter:{{sf_sensors_environmental_type:{{in:["{humidity}"]}}}}' in final


def test_attribute_filter_unmatched_value_returns_empty(session):
    c = gql(session, [("aggregations", AGGS)])
    assert c.findpart('sensor @"Sensor Type":nosuchsensor') == []


def test_attribute_filter_errors_are_loud_not_silent(session):
    c = gql(session, [("aggregations", AGGS)])
    with pytest.raises(ValueError, match="no single SparkFun facet matches 'Nope'") as e:
        c.findpart('sensor @"Nope":x')
    assert "Sensor Type" in str(e.value)                                       # tells you what exists
    with pytest.raises(ValueError, match="negated"):
        c.findpart('sensor -@"Sensor Type":humidity')


def test_unsupported_filters(session):
    c = gql(session, [("products(", SEARCH)])
    for q in ("sensor mfr:sparkfun", "sensor is:rohs", "sensor has:datasheet", "sensor lib:basic", "sensor is:preferred", "sensor has:3dmodel", "mfr:x"):
        with pytest.raises(ValueError):
            c.findpart(q)
    with pytest.raises(ValueError, match="search word"):
        c.findpart("is:instock")
    with pytest.raises(ValueError):
        c.findpart("x", limit=0)


def test_graphql_error_without_data_raises(session):
    c = gql(session, [("products(", {"errors": [{"message": "boom"}]})])
    with pytest.raises(PartsApiError, match="boom"):
        c.findpart("sensor")


def test_partial_graphql_errors_with_data_are_tolerated(session):
    partial = dict(SEARCH, errors=[{"message": "Internal server error", "path": ["products", "items", 0, "custom_attributesV2"]}])
    assert gql(session, [("products(", partial)]).findpart("qwiic led")


def test_query_strings_are_escaped(session):
    c = gql(session, [("products(", SEARCH)])
    c.findpart('say "hi" \\ there', parse=False)
    assert 'search:"say \\"hi\\" \\\\ there"' in queries(session)[0]


def test_facets(session):
    f = gql(session, [("aggregations", AGGS)]).facets("sensor")
    assert f["total"] == AGGS["data"]["products"]["total_count"] and "Sensor Type" in f["attributes"] and "Price" in f["attributes"]
    with pytest.raises(ValueError):
        gql(FakeSession(), []).facets("is:instock")


# ------------------------------------------------------------------ single product
def product_client(session):
    session.add("GET", "sparkfun-qwiic-led-stick-apa102c.html", FakeResponse(200, text=PAGE))
    return gql(session, [("filter:{sku:{eq:", ITEM)])


def test_getpart_reads_documents_from_the_product_page(session):
    p = product_client(session).getpart("COM-18354")
    kinds = [d["kind"] for d in p.extra["documents"]]
    assert "schematic" in kinds and "datasheet" in kinds
    assert p.datasheet_url == "https://cdn.sparkfun.com/datasheets/Components/LED/APA102C.pdf"
    assert all(d["url"].startswith("https://") for d in p.extra["documents"])         # http:// links upgraded
    assert p.price_for(25) == 11.25
    assert 'filter:{sku:{eq:"COM-18354"}}' in queries(session)[0]


def test_getpart_without_page_still_works(session):
    session.add("GET", "sparkfun-qwiic-led-stick-apa102c.html", FakeResponse(500, text="x"))
    p = gql(session, [("filter:{sku:{eq:", ITEM)]).getpart("COM-18354")
    assert p.code == "COM-18354" and p.extra["documents"] == [] and p.datasheet_url is None


def test_unknown_sku_and_validation(session):
    c = gql(session, [("filter:{sku:{eq:", {"data": {"products": {"items": []}}})])
    with pytest.raises(NotFoundError):
        c.getpart("NOPE-1")
    with pytest.raises(NotFoundError):
        c.getprice("NOPE-1")
    with pytest.raises(ValueError):
        c.getpart(" ")


def test_price_stock_attributes_pics_datasheet(session, tmp_path):
    c = product_client(session)
    assert c.getprice("COM-18354", 10) == 11.88 and c.getprice("COM-18354", 1) == 12.5 and c.getprice("COM-18354", 500) == 10.63
    assert len(c.getpartprice("COM-18354")) == 4
    assert c.getstock("COM-18354").in_stock
    assert c.getattributes("COM-18354") == {}
    session.add("GET", "sparkfun.com/media", FakeResponse(200, content=b"JPG"))
    session.add("GET", "cdn.sparkfun.com/datasheets", FakeResponse(200, content=b"%PDF"))
    assert c.getpic("COM-18354", tmp_path).name == "COM-18354.jpg"
    assert len(c.getpics("COM-18354")) >= 2
    assert c.getdatasheet("COM-18354", tmp_path).name == "COM-18354.pdf"


def test_datasheet_missing_lists_other_documents(session):
    only_schematic = '<html><body><li><a href="https://cdn.sparkfun.com/assets/x/Board_Schematic.pdf">Schematic</a></li></body></html>'
    session.add("GET", "sparkfun-qwiic-led-stick-apa102c.html", FakeResponse(200, text=only_schematic))
    c = gql(session, [("filter:{sku:{eq:", ITEM)])
    with pytest.raises(NotFoundError, match="schematic"):
        c.getdatasheet("COM-18354")


def test_no_3d_models(session):
    with pytest.raises(NoModelError):
        SparkFun(make_http(session)).get3dmodel("COM-18354")


def test_requests_identify_as_partsapi():
    assert SparkFun().http.session.headers["User-Agent"].startswith("partsapi/")
