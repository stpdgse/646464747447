import pytest

from conftest import FakeResponse, FakeSession, fixture_json, make_http
from partsapi.easyeda import EasyEDA
from partsapi.errors import NoModelError, NotFoundError, ParseError
from partsapi.lcsc import LCSC

UUID = "ff306e3ed51d4ebc94ebabcd1b4d8906"
STEP = b"ISO-10303-21;\r\nHEADER;"
OBJ = b"v 1.0 2.0 3.0\nv 1 1 1\n"


def ee(session):
    session.add("GET", "/api/products/C2913204/components", FakeResponse(200, json_data=fixture_json("easyeda_component_C2913204.json")))
    return EasyEDA(make_http(session))


def test_model_uuid_extraction():
    comp = fixture_json("easyeda_component_C2913204.json")["result"]
    assert EasyEDA.model_uuid(comp) == UUID
    assert EasyEDA.model_uuid({"packageDetail": {"dataStr": {"shape": ["TRACK~1"], "head": {}}}}) is None


def test_step_and_obj_use_the_right_endpoints(session, tmp_path):
    e = ee(session)
    session.add("GET", f"/3dmodel/{UUID}", FakeResponse(200, content=OBJ))
    session.add("GET", f"qAxj6KHrDKw4blvCG8QJPs7Y/{UUID}", FakeResponse(200, content=STEP))
    step = e.get3dmodel("c2913204", "step", tmp_path)
    obj = e.get3dmodel("C2913204", "obj")
    assert step.data == STEP and step.format == "step" and step.filename == "C2913204.step"
    assert (tmp_path / "C2913204.step").read_bytes() == STEP
    assert obj.data == OBJ and obj.filename == "C2913204.obj"
    assert "EasyEDA" in step.attribution
    assert "ISO-10303" not in repr(step)  # bytes stay out of repr


def test_bad_inputs(session):
    e = ee(session)
    with pytest.raises(ValueError):
        e.get3dmodel("C2913204", "stl")
    with pytest.raises(ValueError):
        e.component("LM358")


def test_empty_model_file_is_no_model(session):
    e = ee(session)
    session.add("GET", f"qAxj6KHrDKw4blvCG8QJPs7Y/{UUID}", FakeResponse(200, content=b""))
    with pytest.raises(NoModelError):
        e.get3dmodel("C2913204", "step")


def test_component_without_3d_node(session):
    comp = fixture_json("easyeda_component_C2913204.json")
    comp["result"]["packageDetail"]["dataStr"]["shape"] = []
    session.add("GET", "/api/products/C1/components", FakeResponse(200, json_data=comp))
    with pytest.raises(NoModelError):
        EasyEDA(make_http(session)).get3dmodel("C1")


def test_unknown_component(session):
    session.add("GET", "/api/products/C9/components", FakeResponse(200, json_data={"success": False}))
    with pytest.raises(NotFoundError):
        EasyEDA(make_http(session)).component("C9")


SEARCH_PAYLOAD = {"success": True, "result": {"lists": {"lcsc": [
    {"uuid": "u1", "title": "LM358_C1", "SMT": True, "dataStr": {"head": {"c_para": {
        "Supplier Part": "C25804", "Manufacturer Part": "LM358", "Manufacturer": "X", "package": "SOP-8", "JLCPCB Part Class": "Basic Part"}}}},
    {"uuid": "u2", "title": "no supplier part", "dataStr": {"head": {"c_para": {}}}},
]}}}


def test_easyeda_search(session):
    session.add("POST", "components/search", FakeResponse(200, json_data=SEARCH_PAYLOAD))
    hits = EasyEDA(make_http(session)).search("LM358")
    assert hits == [{"code": "C25804", "mpn": "LM358", "manufacturer": "X", "package": "SOP-8",
                     "library_class": "Basic Part", "title": "LM358_C1", "smt": True, "uuid": "u1"}]
    assert session.calls[-1]["data"]["wd"] == "LM358"


def test_easyeda_search_failure(session):
    session.add("POST", "components/search", FakeResponse(200, json_data={"success": False}))
    with pytest.raises(ParseError):
        EasyEDA(make_http(session)).search("x")


def lcsc_client(session):
    session.add("GET", "product/detail", FakeResponse(200, json_data=fixture_json("lcsc_detail_C25804.json")))
    return LCSC(make_http(session))


def test_lcsc_getpart(session):
    p = lcsc_client(session).getpart("C25804")
    assert (p.vendor, p.code, p.mpn, p.package) == ("lcsc", "C25804", "0603WAF1002T5E", "0603")
    assert p.category == "Passives / Resistors"
    assert p.manufacturer == "UNI-ROYAL"
    assert p.attributes["Resistance"] == "10kΩ"
    assert p.datasheet_url.startswith("https://datasheet.lcsc.com/")
    assert len(p.image_urls) == 3
    assert p.extra["symbol_svg"].startswith("https://image.easyeda.com/")
    assert p.extra["min_order_qty"] == 100 and p.extra["order_multiple"] == 100
    assert p.stock.quantity == 0 and not p.stock.in_stock


def test_lcsc_price_ladder_ranges(session):
    c = lcsc_client(session)
    b = c.getpartprice("C25804")
    assert [(x.min_qty, x.max_qty) for x in b] == [(100, 999), (1000, 4999), (5000, 9999), (10000, 49999), (50000, 99999), (100000, None)]
    assert c.getpartprice("C25804", qty=1000) == 0.0044
    assert c.getprice("C25804", 100) == 0.0052


def test_lcsc_currency_cookie(session):
    c = lcsc_client(session)
    c.getpart("C25804")
    assert session.calls[-1]["cookies"] is None
    c.currency = "EUR"
    c.getpart("C25804")
    assert session.calls[-1]["cookies"] == {"currencyCode": "EUR"}


def test_lcsc_unknown_part(session):
    session.add("GET", "product/detail", FakeResponse(200, json_data={"code": 500, "msg": "x", "result": None}))
    with pytest.raises(NotFoundError):
        LCSC(make_http(session)).getpart("C1")
    with pytest.raises(ValueError):
        LCSC(make_http(session)).getpart("LM358")


def test_lcsc_findpart_code_shortcut_and_keyword(session):
    c = lcsc_client(session)
    assert [p.code for p in c.findpart("C25804")] == ["C25804"]
    assert not any("components/search" in x["url"] for x in session.calls)
    session.add("POST", "components/search", FakeResponse(200, json_data=SEARCH_PAYLOAD))
    assert [p.code for p in c.findpart("LM358", limit=3)] == ["C25804"]


def test_lcsc_assets(session, tmp_path):
    c = lcsc_client(session)
    session.add("GET", "assets.lcsc.com", FakeResponse(200, content=b"IMG"))
    session.add("GET", "datasheet.lcsc.com", FakeResponse(200, content=b"%PDF"))
    session.add("GET", "image.easyeda.com", FakeResponse(200, content=b"<svg/>"))
    assert c.getpic("C25804", tmp_path).name == "C25804.jpg"
    assert c.getdatasheet("C25804", tmp_path).read_bytes() == b"%PDF"
    assert c.getsymbol("C25804", tmp_path).name == "C25804_symbol.svg"
    assert c.getfootprint("C25804") == b"<svg/>"
