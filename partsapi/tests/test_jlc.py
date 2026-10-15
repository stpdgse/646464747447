import pytest

from conftest import FakeResponse, FakeSession, fixture_json, make_http
from partsapi.errors import NotFoundError, PartsApiError
from partsapi.jlc import JLC

SEARCH = "selectSmtComponentList"


def client(session: FakeSession) -> JLC:
    session.add("POST", SEARCH, FakeResponse(200, json_data=fixture_json("jlc_search_C25804.json")))
    return JLC(make_http(session))


def test_getpart_maps_everything(session):
    p = client(session).getpart("c25804")
    assert (p.vendor, p.code, p.mpn) == ("jlc", "C25804", "0603WAF1002T5E")
    assert p.manufacturer.startswith("UNI-ROYAL")
    assert p.package == "0603"
    assert p.extra["library_type"] == "basic"
    assert p.attributes["Resistance"] == "10kΩ"
    assert p.datasheet_url.endswith("C25804.pdf")
    assert p.stock.quantity > 0 and p.stock.in_stock
    assert p.url.startswith("https://www.lcsc.com/product-detail/")
    assert [b.min_qty for b in p.price_breaks] == [1, 1000, 5000, 10000, 50000, 100000]
    assert p.price_breaks[-1].max_qty is None


def test_price_for_quantity(session):
    c = client(session)
    assert c.getpartprice("C25804", qty=1) == 0.0052
    assert c.getpartprice("C25804", qty=1000) == 0.0043
    assert c.getpartprice("C25804", qty=999999) == 0.0034
    assert len(c.getpartprice("C25804")) == 6
    assert c.getprice("C25804", 5000) == 0.0039


def test_search_body_includes_filters(session):
    c = client(session)
    c.findpart("10k 0603", limit=5, page=2, library="extended", preferred=True, in_stock_only=True,
               min_stock=100, brands=["UNI-ROYAL"], attributes={"Tolerance": "±1%"})
    body = session.calls[-1]["json"]
    assert body["keyword"] == "10k 0603" and body["pageSize"] == 72  # digit words are checked client-side, so read full pages and body["currentPage"] == 2
    assert body["componentLibraryType"] == "expand"
    assert body["preferredComponentFlag"] is True
    assert body["presaleType"] == "stock" and body["startStockNumber"] == 100
    assert body["componentBrandList"] == ["UNI-ROYAL"]
    assert body["componentAttributeList"] == [{"Tolerance": ["±1%"]}]


def test_default_search_includes_out_of_stock(session):
    c = client(session)
    c.findpart("x")
    assert session.calls[-1]["json"]["presaleType"] == ""


def test_input_validation(session):
    c = client(session)
    with pytest.raises(ValueError):
        c.findpart("x", limit=0)
    with pytest.raises(ValueError):
        c.findpart("x", limit=5001)
    with pytest.raises(ValueError):
        c.findpart("x", library="premium")
    with pytest.raises(ValueError):
        c.getpart("not-a-code")


def test_missing_part_is_not_found(session):
    session.add("POST", SEARCH, FakeResponse(200, json_data={"code": 200, "data": {"componentPageInfo": {"list": []}}}))
    with pytest.raises(NotFoundError):
        JLC(make_http(session)).getpart("C1")


def test_api_error_code_raises(session):
    session.add("POST", SEARCH, FakeResponse(200, json_data={"code": 500, "message": "boom"}))
    with pytest.raises(PartsApiError, match="boom"):
        JLC(make_http(session)).findpart("x")


def test_pic_and_datasheet_download_and_save(session, tmp_path):
    c = client(session)
    session.add("GET", "smtComponentImageFile", FakeResponse(200, content=b"\xff\xd8JPEG"))
    session.add("GET", "smtDataManualFile", FakeResponse(200, content=b"%PDF-1"))
    assert c.getpic("C25804") == b"\xff\xd8JPEG"
    path = c.getpic("C25804", save_to=tmp_path)
    assert path.name == "C25804.jpg" and path.read_bytes() == b"\xff\xd8JPEG"
    assert c.getdatasheet("C25804", save_to=tmp_path / "ds.pdf").read_bytes() == b"%PDF-1"


def test_datasheet_falls_back_to_public_url(session):
    c = client(session)
    session.add("GET", "smtDataManualFile", FakeResponse(404))
    session.add("GET", "lcsc_datasheet", FakeResponse(200, content=b"%PDF-public"))
    assert c.getdatasheet("C25804") == b"%PDF-public"
