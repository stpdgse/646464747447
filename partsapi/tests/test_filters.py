import pytest

from partsapi.filters import (Cond, PartFilter, build_filter, facets, parse_quantity, parse_query, sort_parts)
from partsapi.models import Part, PriceBreak, Stock


# ------------------------------------------------------------------ quantities
@pytest.mark.parametrize("text,value,unit", [
    ("10kΩ", 1e4, "Ω"), ("10k", 1e4, ""), ("100 mW", 0.1, "W"), ("±1%", 1, "%"), ("4k7", 4700, ""), ("4R7", 4.7, ""),
    ("1.1 MHz", 1.1e6, "Hz"), ("4.7uF", 4.7e-6, "F"), ("100nF", 1e-7, "F"), ("2,2µF", 2.2e-6, "F"), ("5 mΩ", 5e-3, "Ω"),
    ("75V", 75, "V"), ("32 VDC", 32, "V"), ("2.2M", 2.2e6, ""), ("0603", 603, ""), ("-3.3V", -3.3, "V"), ("0Ω", 0, "Ω"),
])
def test_parse_quantity(text, value, unit):
    q = parse_quantity(text)
    assert q is not None and q.unit == unit and q.value == pytest.approx(value)


@pytest.mark.parametrize("text", ["", None, "abc", "-55℃~+155℃", "3,0 ... 36,0 VDC", "-", "n/a"])
def test_parse_quantity_rejects_non_scalars(text):
    assert parse_quantity(text) is None


# ------------------------------------------------------------------ conditions
@pytest.mark.parametrize("expr,text,ok", [
    (">=10k", "10kΩ", True), (">=10k", "9.9kΩ", False), (">10k", "10kΩ", False), ("<=1%", "±1%", True), ("<=1%", "±5%", False),
    ("10k..100k", "47kΩ", True), ("10k..100k", "100kΩ", True), ("10k..100k", "1MΩ", False),
    ("100n", "100nF", True), ("100n", "0.1uF", True), ("100n", "10nF", False),
    (">=10k", "10nF", False),            # unit mismatch is not "bigger"
    (">=10kΩ", "20k", True),             # part value without unit still compares
    ("ti|onsemi", "TI", True), ("ti|onsemi", "OnSemi Corp", True), ("ti", "Yageo", False),
    ("=10k", "10kΩ", True), ("=10k", "100kΩ", False), ("!=10k", "100kΩ", True), ("~^0603", "0603WAF", True),
    (">=5V", "-", False), (">=5V", "3,0 ... 36,0 VDC", False),
])
def test_cond(expr, text, ok):
    assert Cond.parse(expr).test(text) is ok


def test_cond_negate_parse():
    c = Cond.parse("!yageo")
    assert c.negate and c.test("Yageo")


# ------------------------------------------------------------------ query parser
def test_parse_query_everything():
    pq = parse_query('resistor 0603 "thick film" lm358 OR lm324 OR lm2904 -auto mfr:ti|onsemi res:>=10k pkg:=0603 '
                     '@"Number of Pins":>=8 is:preferred has:datasheet -pkg:0402 USB:C')
    assert pq.keywords == ["resistor", "0603", "thick film", "USB:C"]  # unknown key:value stays a plain word
    assert pq.or_groups == [["lm358", "lm324", "lm2904"]]
    assert len(pq.server_queries()) == 3 and pq.server_queries()[1].endswith("lm324")
    f = pq.filter
    assert f.exclude_terms == ["auto"] and f.flags == {"preferred"} and f.has == {"datasheet"}
    assert ("manufacturer", ) == (f.fields[0][0], ) and f.fields[0][1].operands == ["ti", "onsemi"]
    assert f.fields[2][0] == "package" and f.fields[2][1].negate
    assert [(n, e) for n, _, e in f.attrs] == [("res", False), ("Number of Pins", True)]
    # digit words and phrases are enforced client-side; plain words ("resistor") are left to the vendor
    assert set(f.all_terms) == {"0603", "thick film"}


def test_quoted_values_after_operators():
    f = parse_query('cat:="Chip Resistor - Surface Mount" desc:"dual op" pkg:>="a b" -"two words"').filter
    assert [(n, c.op, c.operands) for n, c in f.fields] == [("category", "eq", ["Chip Resistor - Surface Mount"]),
                                                            ("description", "auto", ["dual op"]), ("package", "ge", ["a b"])]
    assert f.exclude_terms == ["two words"]


def test_negated_exact_attribute_names():
    f = parse_query('-@"Number of Pins":>=8 @"Form Factor":qwiic').filter
    assert [(n, c.negate, e) for n, c, e in f.attrs] == [("Number of Pins", True, True), ("Form Factor", False, True)]


def test_strict_modes():
    assert parse_query("resistor 0603", strict=True).filter.all_terms == ["resistor", "0603"]
    assert parse_query("resistor 0603", strict=False).filter.all_terms == []
    assert parse_query("resistor 0603").filter.all_terms == ["0603"]


def test_or_expansion_cap():
    pq = parse_query("a OR b c OR d e OR f g OR h")
    assert len(pq.server_queries(max_searches=16)) == 16
    with pytest.raises(ValueError):
        pq.server_queries(max_searches=8)


def test_where_string_requires_bare_words():
    f = build_filter("thick res:>=1k -auto", qty=100)
    assert f.all_terms == ["thick"] and f.qty == 100


# ------------------------------------------------------------------ matching
def part(code="C1", mpn="R1", mfr="Uni-Royal", pkg="0603", price=0.01, stock=500, **kw) -> Part:
    return Part(vendor="jlc", code=code, mpn=mpn, manufacturer=mfr, package=pkg, description=kw.pop("description", "Thick Film Resistor"),
                category=kw.pop("category", "Resistors / Chip Resistor"),
                price_breaks=[PriceBreak(1, 99, price), PriceBreak(100, None, price / 2)] if price is not None else [],
                stock=Stock("jlc", code, stock, (stock or 0) > 0) if stock is not None else None,
                attributes=kw.pop("attributes", {"Resistance": "10kΩ", "Tolerance": "±1%", "Power(Watts)": "100mW", "Voltage Rating": "75V"}),
                datasheet_url=kw.pop("datasheet", "http://x/ds.pdf"), image_urls=kw.pop("images", ["http://x/i.jpg"]),
                extra=kw.pop("extra", {"library_type": "basic", "preferred": True, "rohs": True}))


@pytest.mark.parametrize("expr,expected", [
    ("res:>=10k", True), ("res:>10k", False), ("res:10k..100k tol:<=1%", True), ("power:>=0.1W", True), ("power:>=0.25W", False),
    ("volt:>=50V", True), ("volt:>=100V", False), ("mfr:uni", True), ("mfr:=uni", False), ("mfr:=Uni-Royal", True), ("-mfr:uni", False),
    ("pkg:0603", True), ("pkg:=0402", False), ("cat:resistor", True), ("cat:capacitor", False),
    ("lib:basic", True), ("lib:extended", False), ("is:preferred is:instock is:rohs", True), ("is:basic", True), ("is:extended", False),
    ("has:datasheet has:image has:price", True), ("stock:>=500", True), ("stock:>500", False), ("price:<=0.01", True), ("price:<0.01", False),
    ("code:C1", True), ("mpn:R1", True), ("desc:thick", True), ("thick", True), ("-thick", False), ('"thick film"', True), ("thin", False),
    ("thin OR thick", True), ("-thin", True), ("@Resistance:>=10k", True), ('@"Power(Watts)":>=1', False), ("nonexistent:5", False),
])
def test_filter_matches(expr, expected):
    assert build_filter(expr).matches(part()) is expected


def test_price_filter_uses_quantity_ladder():
    p = part(price=0.01)
    assert build_filter("price:<=0.005", qty=100).matches(p)
    assert not build_filter("price:<=0.005", qty=1).matches(p)


def test_missing_data_fails_positive_conditions_but_passes_negated():
    bare = part(price=None, stock=None, datasheet=None, images=[], attributes={})
    for expr in ("res:>=1", "price:<1", "stock:>=1", "has:datasheet", "has:image", "is:instock"):
        assert not build_filter(expr).matches(bare), expr
    assert build_filter("-res:>=1").matches(bare)


def test_in_stock_only_vendor_without_counts():
    reichelt = part(stock=None)
    reichelt.stock = Stock("reichelt", "X", None, True)
    assert build_filter("stock:>0").matches(reichelt)
    assert not build_filter("stock:>=1000").matches(reichelt)
    reichelt.stock = Stock("reichelt", "X", None, False)
    assert not build_filter("stock:>0").matches(reichelt)


def test_text_matching_ignores_spaces_and_dashes():
    p = part(code="LM 358 DIP", mpn=None, description="Operationsverstärker")
    assert build_filter("lm358").matches(p) and build_filter("lm-358-dip").matches(p)
    assert not build_filter("-lm358").matches(p)


def test_skip_groups_for_partial_data():
    stub = part(price=None, stock=None, attributes={}, datasheet=None, images=[])
    f = build_filter("res:>=10k price:<1 has:datasheet mfr:uni")
    assert not f.matches(stub)
    assert f.matches(stub, skip={"attrs", "price", "datasheet"})
    assert f.requires_detail({"manufacturer", "package"})
    assert not build_filter("mfr:uni pkg:0603").requires_detail({"manufacturer", "package", "text"})


def test_merge_and_empty():
    a = build_filter("mfr:uni")
    a.merge(build_filter("res:>=1k -thin"))
    assert len(a.fields) == 1 and len(a.attrs) == 1 and a.exclude_terms == ["thin"] and not a.empty
    assert PartFilter().empty


# ------------------------------------------------------------------ sort / facets
def test_sort_parts():
    ps = [part("C1", price=0.5, stock=10), part("C2", price=0.1, stock=None), part("C3", price=None, stock=99)]
    assert [p.code for p in sort_parts(ps, "price")] == ["C2", "C1", "C3"]           # missing price last
    assert [p.code for p in sort_parts(ps, "price", reverse=True)] == ["C1", "C2", "C3"]
    assert [p.code for p in sort_parts(ps, "stock", reverse=True)] == ["C3", "C1", "C2"]
    assert [p.code for p in sort_parts(ps, lambda p: p.code, reverse=True)] == ["C3", "C2", "C1"]
    with pytest.raises(ValueError):
        sort_parts(ps, "colour")


def test_facets():
    ps = [part("C1", mfr="A", price=0.5), part("C2", mfr="A", pkg="0402", price=0.1), part("C3", mfr="B", price=0.3)]
    f = facets(ps)
    assert f["total"] == 3 and f["manufacturers"][0] == ("A", 2) and dict(f["packages"]) == {"0603": 2, "0402": 1}
    assert f["price_range"] == (0.1, 0.5) and f["in_stock"] == 3 and f["with_datasheet"] == 3
    assert dict(f["attributes"]["Resistance"]) == {"10kΩ": 3}
    assert facets([])["price_range"] is None


@pytest.mark.parametrize("flt,text,ok", [
    ("0603", "0603 (1608 Metric)", True),         # imperial 0603
    ("0603", "0201 (0603 Metric)", False),        # metric 0603 is imperial 0201: a different, smaller part
    ("0201", "0201 (0603 Metric)", True),
    ("0603", "0603", True), ("0603", "R0603", True), ("0603", "0805", False),
    ("1608 Metric", "0603 (1608 Metric)", True),  # longer filters still match the full text
    ("soic", '8-SOIC (0.154", 3.90mm Width)', True),
    ("0603|0805", "0805 (2012 Metric)", True),
])
def test_package_size_rule(flt, text, ok):
    p = part(pkg=text)
    assert build_filter(f"pkg:{flt}" if " " not in flt else f'pkg:"{flt}"').matches(p) is ok
