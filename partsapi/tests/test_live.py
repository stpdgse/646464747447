"""Hits the real websites. Run with:  PARTSAPI_LIVE=1 pytest -m live"""
import pytest

from partsapi import jlc, lcsc, reichelt

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
