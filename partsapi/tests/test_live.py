"""Hits the real websites. Run with:  PARTSAPI_LIVE=1 pytest -m live"""
import pytest

from partsapi import adafruit, digikey, jlc, lcsc, mouser, reichelt, sparkfun, tme

pytestmark = pytest.mark.live


def test_jlc_everything(tmp_path):
    p = jlc.getpart("C25804")
    assert p.mpn == "0603WAF1002T5E" and p.price_breaks and p.stock.quantity is not None
    assert jlc.getpartprice("C25804", qty=100) > 0
    assert jlc.getpic("C25804")[:3] == b"\xff\xd8\xff"
    assert jlc.getdatasheet("C25804")[:4] == b"%PDF"
    assert jlc.findpart("LM358", library="basic", limit=3)
    assert jlc.get3dmodel("C2913204", "step").data.startswith(b"ISO-10303")
    assert b"\nv " in jlc.get3dmodel("C2913204", "obj").data


def test_lcsc_everything():
    p = lcsc.getpart("C7950")
    assert p.mpn == "LM358DR2G" and p.stock.quantity > 0
    assert lcsc.getpartprice("C7950", qty=100) > 0
    assert lcsc.getpic("C7950")[:3] == b"\xff\xd8\xff"
    assert lcsc.getdatasheet("C7950")[:4] == b"%PDF"
    assert lcsc.getsymbol("C7950").lstrip().startswith(b"<")
    assert any(x.mpn == "LM358" for x in lcsc.findpart("LM358", limit=5))


def test_reichelt_everything():
    assert any(p.code == "LM358DT" for p in reichelt.findpart("LM358"))
    p = reichelt.getpart("LM358DT")
    assert p.price_breaks[0].unit_price > 0 and p.stock.in_stock in (True, False)
    assert reichelt.getpic("LM358DT")[:4] in (b"\x89PNG", b"\xff\xd8\xff\xe0", b"\xff\xd8\xff\xe1")
    assert reichelt.get3dmodel("LM358DT").data.startswith(b"ISO-10303")


def test_filters_live():
    # JLCPCB: numeric ranges resolved against the real catalogue, pushed to the server, sorted by price
    rs = jlc.findpart("cat:resistor res:10k..100k tol:<=1% pkg:=0603 lib:basic is:instock", limit=5, sort="price")
    assert rs and all(p.package == "0603" and p.extra["library_type"] == "basic" for p in rs)
    assert all(10_000 <= float(p.attributes["Resistance"].replace("kΩ", "")) * 1000 <= 100_000 for p in rs)
    assert [p.price_for(1) for p in rs] == sorted(p.price_for(1) for p in rs)
    assert jlc.findpart("cat:resistor res:>1000000000000", limit=3) == []
    assert any(p.mpn.startswith("LM358") for p in jlc.findpart("lm358 OR lm324 mfr:ti|onsemi|stmicro stock:>=1000", limit=10))
    f = jlc.facets("", category="capacitor")
    assert f["total"] > 100_000 and "Capacitance" in f["attributes"] and f["counts"]["basic"] > 0
    # LCSC and Reichelt: client-side filters over real results
    ls = lcsc.findpart("lm358 pkg:soic is:instock", limit=3, max_fetch=12)
    assert ls and all("soic" in p.package.lower() and p.stock.in_stock for p in ls)
    rr = reichelt.findpart("LM358 -dip price:<=0.2", sort="price")
    assert rr and all(p.price_for(1) <= 0.2 and "DIP" not in p.code for p in rr)


@pytest.mark.skipif(not digikey.is_configured(), reason="set DIGIKEY_CLIENT_ID / DIGIKEY_CLIENT_SECRET")
def test_digikey_live():
    p = digikey.getpart("LM358DR")
    assert p.mpn == "LM358DR" and p.price_breaks and p.stock.quantity is not None and p.attributes
    assert digikey.getprice("LM358DR", 100) > 0
    out = digikey.findpart("lm358 mfr:texas is:instock price:<1", sort="price", limit=5)
    assert out and all(x.stock.in_stock for x in out)
    r = digikey.findpart('resistor cat:"Chip Resistor - Surface Mount" res:10k..100k tol:<=1% pkg:0603 is:instock', limit=5, sort="price")
    assert r and all("0603" in (x.package or "") and x.stock.in_stock for x in r)
    assert digikey.facets("resistor")["total"] > 1000
    assert digikey.getpic("LM358DR")[:2] in (b"\xff\xd8", b"\x89P")


@pytest.mark.skipif(not mouser.is_configured(), reason="set MOUSER_API_KEY")
def test_mouser_live():
    out = mouser.findpart("LM358 is:instock -dip", limit=5, sort="price")
    assert out and all(x.stock.in_stock for x in out)
    p = mouser.getpart(out[0].code)
    assert p.mpn and p.price_breaks and p.price_for(1) > 0
    assert mouser.getprice(out[0].code, 100) > 0


def test_adafruit_live(tmp_path):
    out = adafruit.findpart("feather rp2040 is:instock", limit=5, sort="price")
    assert out and all("rp2040" in p.description.lower() and p.stock.in_stock for p in out)
    p = adafruit.getpart("732")
    assert "MCP23017" in p.description and p.price_for(10) < p.price_for(1) and p.datasheet_url.endswith(".pdf")
    assert adafruit.getdatasheet("732")[:4] == b"%PDF"
    assert adafruit.getpic("732")[:3] == b"\xff\xd8\xff"
    assert adafruit.findpart("sensor cat:thermistor price:<5", limit=3)


def test_sparkfun_live(tmp_path):
    out = sparkfun.findpart("qwiic sensor is:instock", limit=5, sort="price")
    assert out and all(p.stock.in_stock for p in out)
    hum = sparkfun.findpart('sensor cat:"Environmental Sensors" @"Sensor Type":humidity', limit=5)
    assert hum and all(p.category and p.category.startswith("Sensors") for p in hum)
    p = sparkfun.getpart("COM-18354")
    assert p.price_for(100) < p.price_for(1) and p.datasheet_url.endswith(".pdf")
    assert sparkfun.getdatasheet("COM-18354")[:4] == b"%PDF"
    assert sparkfun.getpic("COM-18354")[:3] == b"\xff\xd8\xff"


@pytest.mark.skipif(not tme.is_configured(), reason="set TME_TOKEN / TME_SECRET")
def test_tme_live(tmp_path):
    p = tme.getpart("LM358DT")
    assert p.mpn == "LM358DT" and p.price_breaks and p.stock.quantity is not None and p.attributes and p.datasheet_url
    assert tme.getprice("LM358DT", 100) > 0 and tme.getdatasheet("LM358DT")[:4] == b"%PDF"
    out = tme.findpart("lm358 is:instock price:<0.5", sort="price", limit=5)
    assert out and all(x.stock.in_stock for x in out)
    r = tme.findpart("resistor res:10k..100k tol:<=1% pkg:0603 is:instock", limit=5, sort="price")
    assert r and all(x.package == "0603" and x.stock.in_stock for x in r)
    assert tme.facets("lm358")["total"] > 10
