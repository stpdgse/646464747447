import pytest

from conftest import FakeResponse, FakeSession, fixture_json, fixture_text, make_http
from partsapi._cache import DiskCache, default_dir
from partsapi.adafruit import Adafruit, _id_of, _parse, _stock, _tiers
from partsapi.errors import NoModelError, NotFoundError

CATALOG = fixture_json("adafruit_catalog_small.json")


def client(session, tmp_path, **kw):
    session.add("GET", "/api/products", FakeResponse(200, json_data=CATALOG))
    session.add("GET", "/api/categories", FakeResponse(200, json_data=fixture_json("adafruit_categories_small.json")))
    session.add("GET", "/api/category/", lambda **k: FakeResponse(200, json_data={"category_name": "Cat " + k["url"].rsplit("/", 1)[1]}))
    return Adafruit(make_http(session), cache_dir=tmp_path, **kw)


def calls(session, part):
    return [c for c in session.calls if part in c["url"]]


# ------------------------------------------------------------------ parsing
@pytest.mark.parametrize("raw,expected", [("in stock", (None, True)), ("94", (94, True)), ("0", (0, False)), ("-3", (0, False)),
                                          ("", (None, False)), (None, (None, False)), ("weird", (None, False))])
def test_stock_semantics(raw, expected):
    assert _stock(raw, False) == expected


def test_tiers_accept_list_or_python_repr_string():
    rows = [{"discounted_price": "6.95", "min_qty": 1}, {"discounted_price": "5.56", "min_qty": 100}, {"discounted_price": "6.26", "min_qty": 10}]
    expected = [(1, 9, 6.95), (10, 99, 6.26), (100, None, 5.56)]
    assert [(b.min_qty, b.max_qty, b.unit_price) for b in _tiers(rows)] == expected
    assert [(b.min_qty, b.max_qty, b.unit_price) for b in _tiers(repr(rows))] == expected
    assert _tiers("not python") == [] and _tiers(None) == []


@pytest.mark.parametrize("code,pid", [("4884", "4884"), ("ADA4884", "4884"), ("ada4884", "4884"),
                                      ("https://www.adafruit.com/product/4884", "4884"), (" 732 ", "732")])
def test_id_parsing(code, pid):
    assert _id_of(code) == pid


def test_id_parsing_rejects_garbage():
    with pytest.raises(ValueError):
        _id_of("lm358")


def test_parse_real_product():
    raw = next(p for p in CATALOG if p["product_id"] == "4884")
    p = _parse(raw, "Feather")
    assert (p.vendor, p.code, p.mpn, p.manufacturer) == ("adafruit", "4884", "ADA4884", "Adafruit")
    assert p.description == "Adafruit Feather RP2040" and p.category == "Feather"
    assert p.stock.quantity == 93 and p.stock.in_stock
    assert p.price_for(1) == 11.95 and [b.min_qty for b in p.price_breaks][0] == 1
    assert p.image_urls[0].endswith("/1200x900/" + p.image_urls[1].rsplit("/", 1)[1]) and "/640x480/" in p.image_urls[1]
    assert p.extra["rohs"] is True and p.extra["discontinued"] is False and p.url == "https://www.adafruit.com/product/4884"


def test_discontinued_is_not_in_stock():
    raw = next(p for p in CATALOG if p["product_id"] == "385")
    p = _parse(raw)
    assert p.extra["discontinued"] is True and p.stock.in_stock is False


# ------------------------------------------------------------------ catalogue download + cache
def test_catalog_is_downloaded_once_and_cached_on_disk(session, tmp_path):
    c = client(session, tmp_path)
    c.findpart("feather")
    c.findpart("mcp23017")
    assert len(calls(session, "/api/products")) == 1
    assert (tmp_path / "adafruit_products.json").exists()
    session2 = FakeSession()
    c2 = client(session2, tmp_path)                       # fresh object, warm disk cache
    assert c2.findpart("feather")
    assert calls(session2, "/api/products") == []


def test_stale_disk_cache_is_refreshed(session, tmp_path):
    import os, time
    c = client(session, tmp_path)
    c.findpart("feather")
    path = tmp_path / "adafruit_products.json"
    old = time.time() - 3 * 86400
    os.utime(path, (old, old))
    session2 = FakeSession()
    client(session2, tmp_path).findpart("feather")
    assert len(calls(session2, "/api/products")) == 1


def test_cache_can_be_disabled(session):
    c = Adafruit(make_http(session), cache_dir=None)
    session.add("GET", "/api/products", FakeResponse(200, json_data=CATALOG))
    session.add("GET", "/api/category/", FakeResponse(200, json_data={"category_name": "x"}))
    assert c.findpart("feather")


def test_cache_dir_env(monkeypatch, tmp_path):
    monkeypatch.setenv("PARTSAPI_CACHE_DIR", str(tmp_path))
    assert default_dir() == tmp_path
    for off in ("", "off", "OFF", "none", "0"):
        monkeypatch.setenv("PARTSAPI_CACHE_DIR", off)
        assert default_dir() is None


def test_disk_cache_roundtrip_and_corruption(tmp_path):
    dc = DiskCache(tmp_path)
    assert dc.get("x.json", 60) is None
    dc.put("x.json", {"a": [1, 2]})
    assert dc.get("x.json", 60) == {"a": [1, 2]}
    (tmp_path / "x.json").write_text("{broken")
    assert dc.get("x.json", 60) is None
    DiskCache(None).put("y", 1)                            # no-op, no error


def test_unexpected_catalog_shape(session, tmp_path):
    session.add("GET", "/api/products", FakeResponse(200, json_data={"error": "x"}))
    with pytest.raises(NotFoundError):
        Adafruit(make_http(session), cache_dir=tmp_path).findpart("feather")


# ------------------------------------------------------------------ search
def test_search_all_words_required_and_ranking(session, tmp_path):
    c = client(session, tmp_path)
    out = c.findpart("feather rp2040")
    assert [p.code for p in out] == ["4884", "5980", "5712"]               # all in stock => alphabetical by name
    assert all("feather" in p.description.lower() and "rp2040" in p.description.lower() for p in out)
    assert [p.code for p in c.findpart("feather rp2040 adalogger")] == ["5980"]
    assert c.findpart("feather esp32") == []


def test_exact_number_ranks_first(session, tmp_path):
    out = client(session, tmp_path).findpart("mcp23017 OR 593")
    assert {p.code for p in out} == {"732", "5346", "593"}


def test_default_hides_discontinued(session, tmp_path):
    c = client(session, tmp_path)
    assert c.findpart("dht22") == []
    assert {p.code for p in c.findpart("dht22", include_discontinued=True)} == {"385", "393"}


def test_filters(session, tmp_path):
    c = client(session, tmp_path)
    assert [p.code for p in c.findpart("feather is:instock price:<15")] == ["4884", "5980"]
    assert [p.code for p in c.findpart("feather stock:>=50")] == ["4884"]          # 93 in stock; 46 is not >= 50
    assert {p.code for p in c.findpart("mcp -breakout")} == {"732", "593"}
    assert [p.code for p in c.findpart("feather", sort="price")][0] == "4884"
    assert [p.code for p in c.findpart("feather", sort="price", reverse=True)][0] == "5712"
    assert [p.code for p in c.findpart("mcp23017", where="price:<6")] == ["5346"]
    # quantity-aware: 10 pieces of the 6.95 MCP23017 cost 6.26 each
    assert [p.code for p in c.findpart("mcp23017 price:<6.3", qty=10)] == ["732", "5346"]


def test_cat_filter_loads_the_tree_once(session, tmp_path):
    c = client(session, tmp_path)
    out = c.findpart('temperature cat:"Thermistor"', limit=10)
    assert {p.code for p in out} == {"4890", "165", "4686"} and all(p.category.endswith("Thermistor") for p in out)
    assert [p.code for p in c.findpart("sensor cat:Sensors", limit=20)]                 # substring on the path
    assert len(calls(session, "/api/categories")) == 1
    assert (tmp_path / "adafruit_category_paths.json").exists()


def test_category_names_for_results_use_cheap_per_id_lookups(session, tmp_path):
    c = client(session, tmp_path)
    out = c.findpart("feather rp2040", limit=3)
    assert calls(session, "/api/categories") == []                                       # the 28 MB tree is not needed
    assert all(p.category and p.category.startswith("Cat ") for p in out)
    n = len(calls(session, "/api/category/"))
    c.findpart("feather rp2040", limit=3)
    assert len(calls(session, "/api/category/")) == n                                    # remembered


def test_unsupported_filters(session, tmp_path):
    c = client(session, tmp_path)
    for q in ("feather has:datasheet", "feather has:3dmodel", "feather is:preferred", "feather lib:basic"):
        with pytest.raises(ValueError):
            c.findpart(q)
    with pytest.raises(ValueError):
        c.findpart("x", limit=0)


def test_facets_local(session, tmp_path):
    f = client(session, tmp_path).facets("feather")
    assert f["total"] == 3 and f["price_range"] == (11.95, 19.95)


# ------------------------------------------------------------------ single product
def product_client(session, tmp_path):
    session.add("GET", "/api/product/732", FakeResponse(200, json_data=fixture_json("adafruit_product_732.json")))
    session.add("GET", "/api/product/3333", FakeResponse(200, json_data=fixture_json("adafruit_product_3333.json")))
    session.add("GET", "/api/product/999", FakeResponse(200, json_data={}))
    session.add("GET", "/product/732", FakeResponse(200, text=fixture_text("adafruit_page_732.html")))
    session.add("GET", "/product/2652", FakeResponse(200, text=fixture_text("adafruit_page_2652.html")))
    session.add("GET", "/product/3333", FakeResponse(404, text="gone"))
    session.add("GET", "/api/category/", FakeResponse(200, json_data={"category_name": "IC & Transistors"}))
    return Adafruit(make_http(session), cache_dir=tmp_path)


def test_getpart_adds_description_and_documents(session, tmp_path):
    p = product_client(session, tmp_path).getpart("ADA732")
    assert p.code == "732" and p.category == "IC & Transistors" and "MCP23017" in p.description
    assert p.extra["summary"].startswith("Add another 16 pins")
    assert p.datasheet_url == "https://cdn-shop.adafruit.com/datasheets/mcp23017.pdf"
    assert p.extra["documents"] == [{"url": "https://cdn-shop.adafruit.com/datasheets/mcp23017.pdf", "title": "Datasheet for MCP23017"}]
    assert [b.min_qty for b in p.price_breaks] == [1, 10, 100]


def test_getpart_survives_a_missing_product_page(session, tmp_path):
    p = product_client(session, tmp_path).getpart("3333")
    assert p.code == "3333" and p.extra["documents"] == [] and p.datasheet_url is None and p.extra["summary"]


def test_key_value_bullets_become_attributes(session, tmp_path):
    raw = dict(fixture_json("adafruit_product_732.json"), products_description="<ul><li>Dimensions: 20mm x 10mm</li><li>Just a sentence, no colon</li><li>Weight: 2g</li></ul>")
    session.add("GET", "/api/product/732", FakeResponse(200, json_data=raw))
    session.add("GET", "/product/732", FakeResponse(404, text="x"))
    session.add("GET", "/api/category/", FakeResponse(200, json_data={"category_name": "x"}))
    assert Adafruit(make_http(session), cache_dir=tmp_path).getattributes("732") == {"Dimensions": "20mm x 10mm", "Weight": "2g"}


def test_getpart_unknown_product(session, tmp_path):
    with pytest.raises(NotFoundError):
        product_client(session, tmp_path).getpart("999")


def test_price_stock_pics_datasheet(session, tmp_path):
    c = product_client(session, tmp_path)
    assert c.getprice("732", 10) == 6.26 and c.getprice("732", 1000) == 5.56 and len(c.getpartprice("732")) == 3
    assert c.getstock("732").in_stock
    session.add("GET", "cdn-shop.adafruit.com/1200x900", FakeResponse(404, text="no"))
    session.add("GET", "cdn-shop.adafruit.com/640x480", FakeResponse(200, content=b"JPG"))
    session.add("GET", "cdn-shop.adafruit.com/datasheets", FakeResponse(200, content=b"%PDF"))
    assert c.getpic("732", tmp_path).read_bytes() == b"JPG"                 # falls back when 1200x900 is missing
    assert c.getdatasheet("732", tmp_path).name == "adafruit_732.pdf"
    assert len(c.getpics("732")) == 2


def test_datasheet_hint_points_to_the_learn_guide(session, tmp_path):
    raw = dict(fixture_json("adafruit_product_732.json"), product_id="2652", product_url="https://www.adafruit.com/product/2652")
    session.add("GET", "/api/product/2652", FakeResponse(200, json_data=raw))
    session.add("GET", "/product/2652", FakeResponse(200, text=fixture_text("adafruit_page_2652.html")))
    session.add("GET", "/api/category/", FakeResponse(200, json_data={"category_name": "x"}))
    with pytest.raises(NotFoundError, match="learn.adafruit.com/.*/downloads"):
        Adafruit(make_http(session), cache_dir=tmp_path).getdatasheet("2652")


def test_no_3d_models(session, tmp_path):
    with pytest.raises(NoModelError):
        client(session, tmp_path).get3dmodel("4884")


def test_requests_identify_as_partsapi(session, tmp_path):
    c = Adafruit(cache_dir=tmp_path)
    assert c.http.session.headers["User-Agent"].startswith("partsapi/")
