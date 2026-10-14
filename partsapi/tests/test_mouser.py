import pytest

from conftest import FakeResponse, FakeSession, fixture_json, make_http
from partsapi.errors import AuthError, BlockedError, NotFoundError, PartsApiError
from partsapi.mouser import Mouser, _parse
from partsapi._http import redact

KW = fixture_json("mouser_keyword_synthetic.json")
KEY = "11111111-2222-3333-4444-555555555555"


def client(session, payload=None, **kw):
    session.add("POST", "/v1/search/keyword", FakeResponse(200, json_data=payload or KW))
    return Mouser(make_http(session), KEY, **kw)


def calls(session, part="/v1/search/keyword"):
    return [c for c in session.calls if part in c["url"]]


# ------------------------------------------------------------------ parsing
def test_parse_part():
    p = _parse(KW["SearchResults"]["Parts"][0])
    assert (p.vendor, p.code, p.mpn, p.manufacturer) == ("mouser", "595-LM358DR", "LM358DR", "Texas Instruments")
    assert p.package == "SOIC-8" and p.category == "Op Amps" and p.url.startswith("https://www.mouser.com/ProductDetail")
    assert [(b.min_qty, b.max_qty, b.unit_price, b.currency) for b in p.price_breaks] == [
        (1, 9, 0.55, "USD"), (10, 99, 0.371, "USD"), (100, 2499, 0.255, "USD"), (2500, None, 0.18, "USD")]
    assert p.stock.quantity == 1234 and p.stock.in_stock and p.stock.delivery == "factory lead time 56 Days"
    assert p.stock.detail["on_order"] == [{"quantity": 5000, "date": "11/12/2026"}]
    assert p.extra["rohs"] is True and p.extra["weight_kg"] == pytest.approx(0.000076) and p.extra["alternate_packagings"] == ["LM358DRG4"]
    assert p.extra["compliance"] == {"USHTS": "8542330001", "ECCN": "EAR99"} and p.extra["min_order_qty"] == 1
    assert p.attributes["Number of Channels"] == "2 Channel" and p.datasheet_url.endswith("lm358.pdf")


def test_repeated_attribute_names_keep_every_value():
    # real Mouser data: "Verpackung" (Packaging) is listed once per packaging option
    raw = {"MouserPartNumber": "X", "ProductAttributes": [
        {"AttributeName": "Verpackung", "AttributeValue": "Reel"}, {"AttributeName": "Verpackung", "AttributeValue": "Cut Tape"},
        {"AttributeName": "Verpackung", "AttributeValue": "MouseReel"}, {"AttributeName": "Verpackung", "AttributeValue": "Reel"},
        {"AttributeName": "Standardpackungsmenge", "AttributeValue": "2500"}, {"AttributeName": "Empty", "AttributeValue": ""}]}
    assert _parse(raw).attributes == {"Verpackung": "Reel, Cut Tape, MouseReel", "Standardpackungsmenge": "2500"}


def test_parse_european_prices_and_missing_stock():
    p = _parse(KW["SearchResults"]["Parts"][1])
    assert [(b.unit_price, b.currency) for b in p.price_breaks] == [(0.125, "EUR"), (0.09, "EUR")]
    assert p.stock.quantity is None and not p.stock.in_stock and p.package == "DIP-8"
    assert _parse(KW["SearchResults"]["Parts"][2]).extra["rohs"] is False
    assert _parse({"MouserPartNumber": "X", "Availability": "12,345 In Stock"}).stock.quantity == 12345
    assert _parse({"MouserPartNumber": "X"}).price_breaks == []


# ------------------------------------------------------------------ transport / errors
def test_request_shape(session):
    client(session).findpart("lm358", limit=10, strict=False)
    c = calls(session)[0]
    assert c["url"] == f"https://api.mouser.com/api/v1/search/keyword?apiKey={KEY}"
    assert c["json"] == {"SearchByKeywordRequest": {"keyword": "lm358", "records": 10, "startingRecord": 0, "searchOptions": "None"}}


def test_missing_key(session, monkeypatch):
    monkeypatch.delenv("MOUSER_API_KEY", raising=False)
    m = Mouser(make_http(session))
    assert not m.is_configured()
    with pytest.raises(AuthError, match="MOUSER_API_KEY"):
        m.findpart("lm358")
    assert session.calls == []


def test_api_errors(session):
    bad = {"Errors": [{"Id": 0, "Code": "Invalid", "Message": "Invalid unique identifier.", "PropertyName": "API Key"}], "SearchResults": None}
    with pytest.raises(AuthError, match="rejected the API key"):
        client(session, bad).findpart("x")
    s2 = FakeSession()
    with pytest.raises(BlockedError, match="rate limit"):
        client(s2, {"Errors": [{"Code": "TooManyRequests", "Message": "Maximum calls per minute exceeded"}]}).findpart("x")
    s3 = FakeSession()
    with pytest.raises(PartsApiError, match="boom"):
        client(s3, {"Errors": [{"Code": "Other", "Message": "boom"}]}).findpart("x")


def test_api_key_is_redacted_in_errors(session):
    session.add("POST", "/v1/search/keyword", FakeResponse(500, text="oops"))
    with pytest.raises(BlockedError) as e:
        Mouser(make_http(session), KEY).findpart("x")
    assert KEY not in str(e.value) and "apiKey=***" in str(e.value)
    assert redact("https://a/b?x=1&apiKey=secret&y=2") == "https://a/b?x=1&apiKey=***&y=2"
    assert redact("https://a/b?client_secret=zzz") == "https://a/b?client_secret=***"


# ------------------------------------------------------------------ search
@pytest.mark.parametrize("query,option", [("lm358", "None"), ("lm358 is:instock", "InStock"), ("lm358 is:rohs", "Rohs"),
                                          ("lm358 is:instock is:rohs", "RohsAndInStock")])
def test_server_side_options(session, query, option):
    client(session).findpart(query)
    assert calls(session)[0]["json"]["SearchByKeywordRequest"]["searchOptions"] == option


def test_client_side_filters(session):
    c = client(session)
    assert [p.code for p in c.findpart("lm358")] == ["595-LM358DR", "511-LM358N", "863-LM358ADR"]
    assert [p.code for p in c.findpart("lm358 mfr:texas|onsemi")] == ["595-LM358DR", "863-LM358ADR"]
    assert [p.code for p in c.findpart("lm358 mfr:=onsemi")] == ["863-LM358ADR"]
    assert [p.code for p in c.findpart("lm358 -dip is:instock")] == ["595-LM358DR", "863-LM358ADR"]
    assert [p.code for p in c.findpart("lm358 price:<0.3")] == ["511-LM358N", "863-LM358ADR"]                   # 0.55 is too dear
    assert [p.code for p in c.findpart("lm358 price:<0.3", qty=100)] == ["595-LM358DR", "511-LM358N", "863-LM358ADR"]  # 0.255 at 100 pcs
    assert [p.code for p in c.findpart("lm358 stock:>=1000")] == ["595-LM358DR"]
    assert [p.code for p in c.findpart("lm358 is:rohs")] == ["595-LM358DR", "511-LM358N"]


def test_sorting_limit(session):
    c = client(session)
    assert [p.code for p in c.findpart("lm358", sort="price")] == ["511-LM358N", "863-LM358ADR", "595-LM358DR"]
    out = c.findpart("lm358", sort="stock", reverse=True, limit=2)
    assert [p.code for p in out] == ["595-LM358DR", "863-LM358ADR"]
    assert calls(session)[-1]["json"]["SearchByKeywordRequest"]["records"] == 50    # sorting reads full pages


def test_paging(session):
    row = KW["SearchResults"]["Parts"][0]
    pages = iter([{"Errors": [], "SearchResults": {"NumberOfResult": 120, "Parts": [dict(row, MouserPartNumber=f"A{i}") for i in range(50)]}},
                  {"Errors": [], "SearchResults": {"NumberOfResult": 120, "Parts": [dict(row, MouserPartNumber=f"B{i}") for i in range(50)]}},
                  {"Errors": [], "SearchResults": {"NumberOfResult": 120, "Parts": [dict(row, MouserPartNumber=f"C{i}") for i in range(20)]}}])
    session.add("POST", "/v1/search/keyword", lambda **kw: FakeResponse(200, json_data=next(pages)))
    out = Mouser(make_http(session), KEY).findpart("x mpn:nomatch", max_pages=5)
    assert out == []
    starts = [c["json"]["SearchByKeywordRequest"]["startingRecord"] for c in calls(session)]
    assert starts == [0, 50, 100]                                # last page was short => stop


def test_max_pages_bounds_calls(session):
    row = KW["SearchResults"]["Parts"][0]
    full = {"Errors": [], "SearchResults": {"NumberOfResult": 10_000, "Parts": [dict(row, MouserPartNumber=f"A{i}") for i in range(50)]}}
    session.add("POST", "/v1/search/keyword", FakeResponse(200, json_data=full))
    Mouser(make_http(session), KEY).findpart("x mpn:nomatch", max_pages=2)
    assert len(calls(session)) == 2


def test_or_runs_separate_searches(session):
    client(session).findpart("lm358 OR lm324", strict=False)
    assert [c["json"]["SearchByKeywordRequest"]["keyword"] for c in calls(session)] == ["lm358", "lm324"]


def test_unsupported_filters(session):
    c = client(session)
    for q in ("lm358 is:preferred", "lm358 lib:basic", "lm358 has:3dmodel", "mfr:ti"):
        with pytest.raises(ValueError):
            c.findpart(q)
    for q in ("lm358 volt:>=32V", "lm358 pkg:soic", 'lm358 @"Package / Case":soic', "resistor res:10k"):
        with pytest.raises(ValueError, match="no technical attributes"):
            c.findpart(q)


def test_facets_local(session):
    f = client(session).facets("lm358")
    assert f["total"] == 3 and dict(f["manufacturers"]) == {"Texas Instruments": 1, "STMicroelectronics": 1, "onsemi": 1}


# ------------------------------------------------------------------ single part
def part_client(session, exact=None, loose=None, keyword=None):
    def partnumber(**kw):
        opt = kw["json"]["SearchByPartRequest"]["partSearchOptions"]
        parts = (exact if opt == "Exact" else loose) or []
        return FakeResponse(200, json_data={"Errors": [], "SearchResults": {"NumberOfResult": len(parts), "Parts": parts}})
    session.add("POST", "/v1/search/partnumber", partnumber)
    session.add("POST", "/v1/search/keyword", FakeResponse(200, json_data={"Errors": [], "SearchResults": {"NumberOfResult": len(keyword or []), "Parts": keyword or []}}))
    return Mouser(make_http(session), KEY)


def test_getpart_exact_first(session):
    p = part_client(session, exact=[KW["SearchResults"]["Parts"][0]]).getpart("595-lm358dr")
    assert p.code == "595-LM358DR"
    assert calls(session, "/v1/search/partnumber")[0]["json"] == {"SearchByPartRequest": {"mouserPartNumber": "595-lm358dr", "partSearchOptions": "Exact"}}
    assert calls(session, "/v1/search/keyword") == []


def test_getpart_matches_manufacturer_part_number_and_ignores_other_hits(session):
    parts = KW["SearchResults"]["Parts"]
    c = part_client(session, exact=[parts[2], parts[1]])
    assert c.getpart("LM358N").code == "511-LM358N"


def test_getpart_falls_back_to_loose_then_keyword(session):
    parts = KW["SearchResults"]["Parts"]
    assert part_client(session, loose=[parts[0]]).getpart("LM358DR").code == "595-LM358DR"
    s2 = FakeSession()
    assert part_client(s2, keyword=[parts[1]]).getpart("lm-358n").code == "511-LM358N"
    with pytest.raises(NotFoundError):
        part_client(FakeSession()).getpart("nothing")
    with pytest.raises(ValueError):
        part_client(FakeSession()).getpart("a|b")


def test_price_stock_assets(session, tmp_path):
    c = part_client(session, exact=[KW["SearchResults"]["Parts"][0]])
    session.add("GET", "mouser.com/images", FakeResponse(200, content=b"JPG"))
    session.add("GET", "ti.com/lit", FakeResponse(200, content=b"%PDF"))
    assert c.getprice("LM358DR", 100) == 0.255 and c.getprice("LM358DR", 1) == 0.55
    assert len(c.getpartprice("LM358DR")) == 4
    assert c.getstock("LM358DR").quantity == 1234
    assert c.getattributes("LM358DR")["Package / Case"] == "SOIC-8"
    assert c.getpic("LM358DR", tmp_path).read_bytes() == b"JPG"
    assert c.getdatasheet("LM358DR", tmp_path).name == "595-LM358DR.pdf"
    assert session.calls[-1]["headers"] == {"Referer": "https://www.mouser.com/"}


def test_manufacturers_list(session):
    session.add("GET", "/v2/search/manufacturerlist", FakeResponse(200, json_data={"MouserManufacturerList": {"Count": 2, "ManufacturerList": [{"ManufacturerName": "TI"}, {"ManufacturerName": "onsemi"}]}, "Errors": []}))
    assert Mouser(make_http(session), KEY).manufacturers() == ["TI", "onsemi"]
    assert session.calls[0]["url"].endswith(f"manufacturerlist?apiKey={KEY}")
