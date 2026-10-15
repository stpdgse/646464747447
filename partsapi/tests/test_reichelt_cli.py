import json

import pytest

from conftest import FakeResponse, FakeSession, fixture_json, fixture_text, make_http
from partsapi import cli, jlc
from partsapi.errors import NoModelError, NotFoundError, ParseError
from partsapi.jlc import JLC
from partsapi.reichelt import Reichelt, parse_price

PRODUCT_URL = "operationsverstaerker_2-fach_1_1_mhz_0_6_v_s_soic-8-348784"


@pytest.mark.parametrize("text,expected", [("0,15 €", 0.15), ("1.234,50 €", 1234.5), ("ab 2,99 €", 2.99),
                                           ("10 €", 10.0), ("n/a", None)])
def test_parse_price(text, expected):
    assert parse_price(text) == expected


def reichelt(session) -> Reichelt:
    session.add("GET", "/shop/suche/", FakeResponse(200, text=fixture_text("reichelt_search_LM358.html")))
    session.add("GET", "/shop/produkt/", FakeResponse(200, text=fixture_text("reichelt_product_LM358DT.html")))
    return Reichelt(make_http(session))


def test_search_cards(session):
    parts = reichelt(session).findpart("LM358", strict=False)
    assert len(parts) == 10
    assert len(reichelt(FakeSession()).findpart("LM358")) == 5  # default: "LMV 358" is not "LM358"
    p = next(x for x in parts if x.code == "LM358DT")
    assert p.manufacturer == "STMICROELECTRONICS"  # card quirk: itemprop=sku is the manufacturer
    assert p.price_for(1) == 0.15 and p.price_breaks[0].currency == "EUR" and p.price_breaks[0].incl_vat
    assert p.attributes["Bauform"] == "SOIC-8"
    assert p.url.endswith(PRODUCT_URL)
    assert p.image_urls and p.image_urls[0].startswith("https://cdn-reichelt.de/")
    assert len(reichelt(FakeSession()).findpart("LM358", limit=3, strict=False)) == 3


def test_search_url_is_encoded(session):
    reichelt(session).findpart("10k 0603/1%")
    assert session.calls[-1]["url"].endswith("/de/de/shop/suche/10k%200603%2F1%25")
    with pytest.raises(ValueError):
        reichelt(session).findpart("  ")


def test_getpart_full(session):
    p = reichelt(session).getpart("LM358DT")
    assert (p.vendor, p.code, p.mpn, p.manufacturer) == ("reichelt", "LM358DT", "LM358DT", "STMICROELECTRONICS")
    assert p.extra["ean"] == "9900003487840"
    assert p.stock.in_stock and p.stock.quantity is None and "Werktage" in p.stock.delivery
    assert p.price_breaks[0].unit_price == 0.15 and p.price_breaks[0].vat_rate == 0.19
    assert p.datasheet_url == "https://cdn-reichelt.de/documents/datenblatt/A200/LM158.pdf"
    assert "xxl_ws" in p.image_urls[0]
    assert p.attributes["RoHS"] == "konform"
    assert p.category.startswith("Bauelemente")


def test_getpart_accepts_url_and_uses_cached_url(session):
    r = reichelt(session)
    r.getpart("LM358DT")
    n = len(session.calls)
    r.getpart("lm358dt")  # second lookup must not search again
    assert not any("/suche/" in c["url"] for c in session.calls[n:])
    assert r.getpart("https://www.reichelt.de/de/de/shop/produkt/" + PRODUCT_URL).code == "LM358DT"


def test_price_stock_attributes(session):
    r = reichelt(session)
    assert r.getprice("LM358DT", qty=500) == 0.15
    assert r.getpartprice("LM358DT")[0].unit_price == 0.15
    assert r.getstock("LM358DT").delivery.startswith("ab Lager")
    assert r.getattributes("LM358DT")["Hersteller"] == "STMICROELECTRONICS"


def test_unknown_article_lists_candidates(session):
    with pytest.raises(NotFoundError, match="LM358DT"):
        reichelt(session).getpart("DOES-NOT-EXIST")


def test_non_product_page_is_parse_error(session):
    session.add("GET", "/shop/produkt/", FakeResponse(200, text="<html><body>hi</body></html>"))
    with pytest.raises(ParseError):
        Reichelt(make_http(session)).getpart("https://www.reichelt.de/de/de/shop/produkt/x-1")


def test_assets(session, tmp_path):
    r = reichelt(session)
    session.add("GET", "cdn-reichelt.de/resize", FakeResponse(200, content=b"PNG"))
    session.add("GET", "cdn-reichelt.de/bilder", FakeResponse(200, content=b"JPG"))
    session.add("GET", "datenblatt", FakeResponse(200, content=b"%PDF"))
    assert r.getpic("LM358DT", tmp_path).read_bytes() == b"PNG"
    assert r.getpic("LM358DT", index=1) == b"JPG"
    assert r.getdatasheet("LM358DT", tmp_path).name == "LM358DT.pdf"


def test_get3dmodel_matches_mpn_exactly(session, tmp_path):
    r = reichelt(session)
    payload = {"success": True, "result": {"lists": {"lcsc": [
        {"uuid": "a", "dataStr": {"head": {"c_para": {"Supplier Part": "C111", "Manufacturer Part": "LM358DT-X"}}}},
        {"uuid": "b", "dataStr": {"head": {"c_para": {"Supplier Part": "C9418", "Manufacturer Part": "LM358DT"}}}},
    ]}}}
    session.add("POST", "components/search", FakeResponse(200, json_data=payload))
    comp = fixture_json("easyeda_component_C2913204.json")
    session.add("GET", "/api/products/C9418/components", FakeResponse(200, json_data=comp))
    session.add("GET", "qAxj6KHrDKw4blvCG8QJPs7Y/", FakeResponse(200, content=b"ISO-10303-21;"))
    m = r.get3dmodel("LM358DT", "step", tmp_path)
    assert m.source_code == "C9418"  # C111 (different MPN) was not used
    assert (tmp_path / "C9418.step").exists()


def test_get3dmodel_no_exact_match(session):
    r = reichelt(session)
    session.add("POST", "components/search", FakeResponse(200, json_data={"success": True, "result": {"lists": {"lcsc": []}}}))
    with pytest.raises(NoModelError):
        r.get3dmodel("LM358DT")


def test_cli_end_to_end(session, monkeypatch, capsys):
    session.add("POST", "selectSmtComponentList", FakeResponse(200, json_data=fixture_json("jlc_search_C25804.json")))
    monkeypatch.setattr(jlc, "_default", JLC(make_http(session)))
    assert cli.main(["jlc", "getpartprice", "C25804", "--qty", "1000"]) == 0
    assert json.loads(capsys.readouterr().out) == 0.0043
    assert cli.main(["jlc", "findpart", "x", "--library", "basic", "--limit", "5"]) == 0
    out = json.loads(capsys.readouterr().out)
    assert out[0]["code"] == "C25804" and "raw" not in out[0]


def test_cli_error_exit_code(session, monkeypatch, capsys):
    session.add("POST", "selectSmtComponentList", FakeResponse(200, json_data={"code": 200, "data": {"componentPageInfo": {"list": []}}}))
    monkeypatch.setattr(jlc, "_default", JLC(make_http(session)))
    assert cli.main(["jlc", "getpart", "C1"]) == 1
    assert "no part C1" in capsys.readouterr().err
