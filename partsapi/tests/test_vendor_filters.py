import copy
import json

import pytest

from conftest import FakeResponse, FakeSession, fixture_json, fixture_text, make_http
from partsapi import cli, jlc, parts
from partsapi.jlc import JLC
from partsapi.lcsc import LCSC
from partsapi.reichelt import Reichelt

SEARCH = "selectSmtComponentList"
FILTER = "filterComponentAttribute"
ITEM = fixture_json("jlc_search_C25804.json")["data"]["componentPageInfo"]["list"][0]


def row(code, price=0.01, **over):
    r = copy.deepcopy(ITEM)
    r.update(componentCode=code, componentModelEn=f"MPN-{code}", **over)
    r["componentPrices"] = [{"startNumber": 1, "endNumber": -1, "productPrice": price}]
    return r


def page(rows, total=None):
    return FakeResponse(200, json_data={"code": 200, "data": {"componentPageInfo": {"total": total if total is not None else len(rows), "list": rows}}})


def jlc_client(session, rows=None):
    session.add("POST", SEARCH, page(rows if rows is not None else [ITEM]))
    return JLC(make_http(session))


# ------------------------------------------------------------------ JLC pushdown / paging
def test_jlc_pushes_supported_filters_to_server(session):
    jlc_client(session).findpart("lib:basic is:instock is:preferred stock:>=1000 pkg:=0603|0402 mfr:=UNI-ROYAL has:datasheet has:image resistor")
    b = session.calls[-1]["json"]
    assert b["componentLibraryType"] == "base" and b["presaleType"] == "stock" and b["preferredComponentFlag"] is True
    assert b["startStockNumber"] == 1000 and b["componentSpecificationList"] == ["0603", "0402"]
    assert b["componentBrandList"] == ["UNI-ROYAL"] and b["dateSheet"] is True and b["photo"] is True
    assert b["keyword"] == "resistor" and b["pageSize"] == 72


def test_jlc_only_server_filters_means_exact_page_size(session):
    jlc_client(session).findpart("resistor", limit=5)
    assert session.calls[-1]["json"]["pageSize"] == 5


def test_jlc_negated_and_fuzzy_conditions_stay_client_side(session):
    jlc_client(session).findpart("resistor -pkg:0603 mfr:uni")
    b = session.calls[-1]["json"]
    assert b["componentSpecificationList"] == [] and b["componentBrandList"] == []


def test_jlc_explicit_kwargs_and_category(session):
    jlc_client(session).findpart("x", library="extended", preferred=False, brands=["A"], packages=["P"],
                                 attributes={"Tolerance": "±1%"}, assembly_ok=True, min_stock=5, parse=False)
    b = session.calls[-1]["json"]
    assert b["componentLibraryType"] == "expand" and b["preferredComponentFlag"] is False and b["assemblyComponentFlag"] is True
    assert b["componentAttributeList"] == [{"Tolerance": ["±1%"]}] and b["startStockNumber"] == 5 and b["keyword"] == "x"


def test_jlc_or_runs_one_search_per_alternative_and_dedupes(session):
    c = jlc_client(session)
    out = c.findpart("lm358 OR lm324", strict=False)
    kws = [x["json"]["keyword"] for x in session.calls if SEARCH in x["url"]]
    assert kws == ["lm358", "lm324"] and len(out) == 1  # same code from both searches appears once


def test_jlc_pages_until_enough_matches(session):
    pages = {1: [row(f"C{i}", 0.5) for i in range(72)], 2: [row("C900", 0.001)] + [row(f"C{i}", 0.5) for i in range(100, 171)], 3: [row("C901", 0.002)]}
    session.add("POST", SEARCH, lambda **kw: page(pages[kw["json"]["currentPage"]], total=500))
    out = JLC(make_http(session)).findpart("price:<0.01", limit=2)
    assert [p.code for p in out] == ["C900", "C901"]
    assert [x["json"]["currentPage"] for x in session.calls] == [1, 2, 3]


def test_jlc_respects_max_pages_and_stops_on_short_page(session):
    session.add("POST", SEARCH, lambda **kw: page([row("C1", 0.5)] * 3, total=10_000))
    c = JLC(make_http(session))
    assert c.findpart("price:<0.01", limit=5, max_pages=4) == []
    assert len(session.calls) == 1  # first page was short (<72) => no further pages


def test_jlc_sort_reads_all_pages_then_cuts(session):
    rows = [row("C1", 0.30), row("C2", 0.10), row("C3", 0.20)]
    out = jlc_client(session, rows).findpart("resistor", limit=2, sort="price")
    assert [p.code for p in out] == ["C2", "C3"]
    assert [p.code for p in jlc_client(FakeSession(), rows).findpart("resistor", limit=2, sort="price", reverse=True)] == ["C1", "C3"]


def test_jlc_strict_words(session):
    c = jlc_client(session)
    assert [p.code for p in c.findpart("resistor 0603")] == ["C25804"]
    assert c.findpart("resistor 0402") == []                      # digit word not in the part text
    assert [p.code for p in c.findpart("resistor 0402", strict=False)] == ["C25804"]
    assert c.findpart("zzz", strict=True) == []


def test_jlc_where_merges_with_query(session):
    c = jlc_client(session)
    assert c.findpart("resistor", where="res:>=10k lib:basic")
    assert c.findpart("resistor", where="res:>=1M") == []


# ------------------------------------------------------------------ JLC category scope + attribute resolution
def tree_payload():
    agg = lambda k, n, sub=None: {"key": k, "name": n, "docCount": 1, "displayFlag": True, "subAggs": sub}  # noqa: E731
    return {"code": 200, "data": {"productTypeAggs": [agg("1", "Resistors", [agg("11", "Chip Resistor - Surface Mount")]),
                                                      agg("2", "Capacitors", [agg("21", "MLCC")])]}}


def facet_payload(body):
    base = body["baseQueryDto"]
    if not base["productTypeIdList"] and not base["componentTypeIdList"]:
        return tree_payload()
    vals = [("-", 5), ("1kΩ", 3), ("10kΩ", 9), ("47kΩ", 4), ("1MΩ", 2)]
    return {"code": 200, "data": {"total": 99, "productTypeAggs": [], "componentBrandList": [{"name": "UNI", "docCount": 4, "displayFlag": True}],
                                  "componentSpecificationList": [], "basePart": 1, "preferredPart": 2, "extendPart": 3, "dataSheetPart": 4, "photo": 5,
                                  "parentParamList": [{"name": "Resistance", "docCount": 9, "subAggs": [{"name": n, "docCount": c} for n, c in vals]},
                                                      {"name": "Voltage Rating", "docCount": 1, "subAggs": [{"name": "50V", "docCount": 1}]},
                                                      {"name": "Voltage Rated", "docCount": 1, "subAggs": [{"name": "25V", "docCount": 1}]}]}}


def scoped_client(session):
    session.add("POST", FILTER, lambda **kw: FakeResponse(200, json_data=facet_payload(kw["json"])))
    session.add("POST", SEARCH, page([ITEM]))
    return JLC(make_http(session))


def test_jlc_numeric_attribute_becomes_exact_server_values(session):
    scoped_client(session).findpart("cat:resistor res:>=10k")
    b = [x for x in session.calls if SEARCH in x["url"]][-1]["json"]
    assert b["componentAttributeList"] == [{"Resistance": ["10kΩ", "47kΩ", "1MΩ"]}]
    assert b["firstSortName"] == "Resistors" and b.get("secondSortName") is None


def test_jlc_subcategory_alias_resolves_to_pair(session):
    scoped_client(session).findpart("cat:resistor res:10k..50k", subcategory=None)
    c = scoped_client(FakeSession())
    assert c._find_category("Chip Resistor - Surface Mount") == ("Resistors", "Chip Resistor - Surface Mount")
    assert c._find_category("capacitor") == ("Capacitors", None)
    assert c._find_category("no such thing") is None


def test_jlc_impossible_attribute_returns_without_searching(session):
    c = scoped_client(session)
    assert c.findpart("cat:resistor res:>1G") == []
    assert not any(SEARCH in x["url"] for x in session.calls)


def test_jlc_ambiguous_attribute_name_is_left_to_client(session):
    c = scoped_client(session)
    c.findpart("cat:resistor volt:>=1V")           # matches "Voltage Rating" AND "Voltage Rated"
    b = [x for x in session.calls if SEARCH in x["url"]][-1]["json"]
    assert b["componentAttributeList"] == []


def test_jlc_without_scope_attributes_filter_client_side(session):
    c = jlc_client(session)
    assert c.findpart("resistor res:>=10k") and c.findpart("resistor res:>=1M") == []
    assert session.calls[-1]["json"]["componentAttributeList"] == []


def test_jlc_facets(session):
    f = scoped_client(session).facets("", category="resistor")
    assert f["total"] == 99 and f["brands"] == [("UNI", 4)] and f["counts"]["preferred"] == 2
    assert f["attributes"]["Resistance"][1] == ("1kΩ", 3)
    sent = [x for x in session.calls if FILTER in x["url"]][-1]["json"]["baseQueryDto"]
    assert sent["productTypeIdList"] == ["1"]
    from partsapi.errors import NotFoundError
    with pytest.raises(NotFoundError):
        scoped_client(FakeSession()).facets("", category="no-such-category")


# ------------------------------------------------------------------ LCSC
EE = {"success": True, "result": {"lists": {"lcsc": [
    {"uuid": "a", "title": "t1", "dataStr": {"head": {"c_para": {"Supplier Part": "C25804", "Manufacturer Part": "0603WAF1002T5E", "Manufacturer": "UNI-ROYAL", "package": "R0603", "JLCPCB Part Class": "Basic Part"}}}},
    {"uuid": "b", "title": "t2", "dataStr": {"head": {"c_para": {"Supplier Part": "C111", "Manufacturer Part": "OTHER", "Manufacturer": "ACME", "package": "R0805", "JLCPCB Part Class": "Extended Part"}}}},
]}}}


def lcsc_client(session):
    session.add("POST", "components/search", FakeResponse(200, json_data=EE))
    session.add("GET", "product/detail", FakeResponse(200, json_data=fixture_json("lcsc_detail_C25804.json")))
    return LCSC(make_http(session))


def detail_calls(session):
    return [x for x in session.calls if "product/detail" in x["url"]]


def test_lcsc_prefilter_avoids_fetching_non_matches(session):
    out = lcsc_client(session).findpart("resistor mfr:uni lib:basic", limit=5)
    assert [p.code for p in out] == ["C25804"] and out[0].extra["library_type"] == "basic"
    assert len(detail_calls(session)) == 1                    # ACME hit never fetched


def test_lcsc_full_filters_after_fetch(session):
    c = lcsc_client(session)
    assert c.findpart("resistor", where="res:>=10k pkg:0603")        # attribute + package from the detail
    assert c.findpart("resistor", where="res:>=1M") == []


def test_lcsc_max_fetch_and_dedupe(session):
    c = lcsc_client(session)
    c.findpart("resistor OR capacitor", max_fetch=1)
    assert len(detail_calls(session)) == 1


def test_lcsc_unsupported_inputs(session):
    c = lcsc_client(session)
    with pytest.raises(ValueError, match="preferred"):
        c.findpart("resistor is:preferred")
    with pytest.raises(ValueError, match="search word"):
        c.findpart("res:>=1k")


def test_lcsc_facets_local(session):
    f = lcsc_client(session).facets("resistor mfr:uni")
    assert f["total"] == 1 and f["manufacturers"] == [("UNI-ROYAL", 1)]


# ------------------------------------------------------------------ Reichelt
def reichelt_client(session):
    session.add("GET", "/shop/suche/", FakeResponse(200, text=fixture_text("reichelt_search_LM358.html")))
    session.add("GET", "/shop/produkt/", FakeResponse(200, text=fixture_text("reichelt_product_LM358DT.html")))
    return Reichelt(make_http(session))


def test_reichelt_card_level_filters_need_no_product_pages(session):
    r = reichelt_client(session)
    out = r.findpart("LM358 pkg:soic price:<=0.15 -dip", sort="price")
    assert out and all("soic" in (p.package or "").lower() and p.price_for(1) <= 0.15 for p in out)
    assert not any("/shop/produkt/" in x["url"] for x in session.calls)


def test_reichelt_detail_filters_fetch_product_pages(session):
    out = reichelt_client(session).findpart("LM358 mfr:st has:datasheet is:instock", max_fetch=5)
    assert [p.code for p in out] == ["LM358DT"] and out[0].datasheet_url.endswith("LM158.pdf")


def test_reichelt_unsupported_and_empty(session):
    r = reichelt_client(session)
    with pytest.raises(ValueError):
        r.findpart("LM358 lib:basic")
    with pytest.raises(ValueError):
        r.findpart("  ")
    with pytest.raises(ValueError):
        r.findpart("pkg:soic")


def test_reichelt_spaced_part_numbers_match(session):
    codes = [p.code for p in reichelt_client(session).findpart("LM358", limit=50)]
    assert "LM358DT" in codes and any(c.startswith("LM 358") for c in codes)


def test_reichelt_facets(session):
    f = reichelt_client(session).facets("LM358")
    assert f["total"] >= 5 and f["price_range"][0] > 0


# ------------------------------------------------------------------ cross-vendor + CLI
def test_parts_findpart_reports_unsupported_filters_as_warnings(session, monkeypatch):
    monkeypatch.setattr(jlc, "_default", jlc_client(session))
    monkeypatch.setattr(parts.reichelt, "_default", reichelt_client(FakeSession()))
    with pytest.warns(UserWarning, match="reichelt"):
        out = parts.findpart("resistor lib:basic", vendors=("jlc", "reichelt"))
    assert out["jlc"] and out["reichelt"] == []


def args(*argv):
    return cli.build_parser().parse_args(list(argv))


def test_cli_build_where():
    a = args("jlc", "findpart", "x", "--mfr", "ti", "--mfr", "onsemi", "--package", "0603", "--category", "resistor",
             "--attr", "resistance=>=10k", "--attr", "Number of Pins=>=8", "--exclude", "thick film", "--in-stock", "--min-stock", "100",
             "--max-price", "0.5", "--preferred", "--has-datasheet", "--where", "tol:<=1%")
    w = cli.build_where(a)
    assert w.split()[0] == "tol:<=1%"
    for piece in ("mfr:ti|onsemi", "pkg:=0603", "cat:resistor", "resistance:>=10k", '@"Number of Pins":>=8', '-"thick film"',
                  "is:instock", "stock:>=100", "price:<=0.5", "is:preferred", "has:datasheet"):
        assert piece in w, piece
    # and the generated expression round-trips through the parser
    from partsapi.filters import build_filter
    f = build_filter(w)
    assert f.flags == {"instock", "preferred"} and f.exclude_terms == ["thick film"] and len(f.attrs) == 3


def test_cli_table_and_filters_end_to_end(session, monkeypatch, capsys):
    monkeypatch.setattr(jlc, "_default", jlc_client(session, [row("C1", 0.3), row("C2", 0.1)]))
    assert cli.main(["jlc", "findpart", "resistor", "--sort", "price", "--table", "--max-price", "0.2"]) == 0
    lines = capsys.readouterr().out.strip().splitlines()
    assert lines[0].split()[:2] == ["code", "mpn"] and len(lines) == 3 and lines[2].startswith("C2")
    b = session.calls[-1]["json"]
    assert b["pageSize"] == 72                                   # client-side price filter => full pages
    assert cli.main(["jlc", "findpart", "resistor", "--library", "basic", "--desc", "--sort", "price"]) == 0
    assert json.loads(capsys.readouterr().out)[0]["code"] == "C1"


def test_cli_jlc_facets(session, monkeypatch, capsys):
    monkeypatch.setattr(jlc, "_default", scoped_client(session))
    assert cli.main(["jlc", "facets", "", "--category", "resistor"]) == 0
    assert json.loads(capsys.readouterr().out)["total"] == 99


# ------------------------------------------------------------------ DigiKey / Mouser in the cross-vendor layer and CLI
def test_default_vendors_include_key_gated_ones_only_when_configured(monkeypatch):
    monkeypatch.setattr(parts.digikey._default, "client_id", None)
    monkeypatch.setattr(parts.mouser._default, "api_key", None)
    assert parts.default_vendors() == ("jlc", "lcsc", "reichelt")
    monkeypatch.setattr(parts.mouser._default, "api_key", "k")
    assert parts.default_vendors() == ("jlc", "lcsc", "reichelt", "mouser")
    monkeypatch.setattr(parts.digikey._default, "client_id", "a")
    monkeypatch.setattr(parts.digikey._default, "client_secret", "b")
    assert parts.default_vendors() == ("jlc", "lcsc", "reichelt", "digikey", "mouser")


def test_explicit_unconfigured_vendor_warns_instead_of_raising(monkeypatch):
    monkeypatch.setattr(parts.digikey._default, "client_id", None)
    with pytest.warns(UserWarning, match="digikey: DigiKey needs DIGIKEY_CLIENT_ID"):
        assert parts.findpart("lm358", vendors=("digikey",)) == {"digikey": []}


def test_parts_prices_reports_auth_error_rows(monkeypatch):
    monkeypatch.setattr(parts.mouser._default, "api_key", None)
    rows = parts.prices("LM358", vendors=("mouser",))
    assert rows[0]["vendor"] == "mouser" and "MOUSER_API_KEY" in rows[0]["error"]


def test_cli_credential_and_filter_errors(monkeypatch, capsys):
    monkeypatch.setattr(parts.digikey._default, "client_id", None)
    assert cli.main(["digikey", "findpart", "lm358"]) == 1
    assert "DIGIKEY_CLIENT_ID" in capsys.readouterr().err
    monkeypatch.setattr(parts.mouser._default, "api_key", "k")
    assert cli.main(["mouser", "findpart", "lm358", "--preferred"]) == 2          # unsupported filter => ValueError => exit 2
    assert "does not support is:preferred" in capsys.readouterr().err
    with pytest.raises(SystemExit):
        cli.main(["digikey", "findpart", "x", "--library", "basic"])


def test_cli_mouser_findpart_end_to_end(session, monkeypatch, capsys):
    from partsapi import mouser as mouser_module
    from partsapi.mouser import Mouser
    session.add("POST", "/v1/search/keyword", FakeResponse(200, json_data=fixture_json("mouser_keyword_synthetic.json")))
    monkeypatch.setattr(mouser_module, "_default", Mouser(make_http(session), "k"))
    assert cli.main(["mouser", "findpart", "lm358", "--exclude", "dip", "--max-price", "0.3", "--sort", "price", "--table"]) == 0
    lines = capsys.readouterr().out.strip().splitlines()
    assert [l.split()[0] for l in lines[2:]] == ["863-LM358ADR"]


# ------------------------------------------------------------------ Adafruit / SparkFun in the cross-vendor layer and CLI
def test_adafruit_and_sparkfun_are_opt_in_for_cross_vendor_calls():
    assert "adafruit" not in parts.default_vendors() and "sparkfun" not in parts.default_vendors()
    assert set(parts.VENDORS) >= {"adafruit", "sparkfun"}


def test_cli_adafruit_end_to_end(session, monkeypatch, capsys, tmp_path):
    from partsapi import adafruit as adafruit_module
    from partsapi.adafruit import Adafruit
    session.add("GET", "/api/products", FakeResponse(200, json_data=fixture_json("adafruit_catalog_small.json")))
    session.add("GET", "/api/category/", FakeResponse(200, json_data={"category_name": "Feather"}))
    monkeypatch.setattr(adafruit_module, "_default", Adafruit(make_http(session), cache_dir=tmp_path))
    assert cli.main(["adafruit", "findpart", "feather", "--max-price", "15", "--sort", "price", "--table"]) == 0
    lines = capsys.readouterr().out.strip().splitlines()
    assert [l.split()[0] for l in lines[2:]] == ["4884", "5980"]
    assert cli.main(["adafruit", "findpart", "feather", "--has-datasheet"]) == 2           # unsupported => clean error, exit 2
    assert "does not support has:datasheet" in capsys.readouterr().err


def test_cli_sparkfun_end_to_end(session, monkeypatch, capsys):
    from partsapi import sparkfun as sparkfun_module
    from partsapi.sparkfun import SparkFun
    session.add("POST", "/graphql", FakeResponse(200, json_data=fixture_json("sparkfun_search.json")))
    monkeypatch.setattr(sparkfun_module, "_default", SparkFun(make_http(session)))
    assert cli.main(["sparkfun", "findpart", "qwiic led", "--in-stock", "--sort", "price", "--table"]) == 0
    out = capsys.readouterr().out.strip().splitlines()
    assert out[0].split()[:2] == ["code", "mpn"] and len(out) >= 3
    assert cli.main(["sparkfun", "findpart", "qwiic", "--mfr", "x"]) == 2
    assert "mfr" in capsys.readouterr().err
    with pytest.raises(SystemExit):
        cli.main(["sparkfun", "findpart", "x", "--library", "basic"])
