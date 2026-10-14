"""Search terms, filters, sorting and facets that work the same on every vendor.

Query syntax (everything is optional and combinable)::

    resistor 0603                      all words must match (AND)
    "thick film"                       exact phrase
    lm358 OR lm324                     alternatives (OR)
    -automotive  -"low power"          exclude a word / phrase
    mfr:ti|onsemi                      manufacturer contains "ti" OR "onsemi"
    pkg:0603  pkg:=SOIC-8              package contains / equals
    cat:resistor   mpn:LM358   code:C25  desc:"dual op"
    lib:basic  is:preferred  is:instock  is:rohs  has:datasheet  has:image
    stock:>=1000   price:<0.01         numeric (price = unit price at the given qty)
    res:>=10k  res:10k..100k           attributes, with units understood
    cap:100n   tol:<=1%   power:>=0.25W   volt:>=50V
    @"Number of Pins":>=8              any attribute by its exact (case-insensitive) name
    mfr:!yageo   -pkg:0402             negation

Operators: ``>= <= > < = !=``, ranges ``a..b``, alternatives ``a|b``, regex ``~pattern``.
Numbers carry units and SI prefixes: ``10k`` = ``10kΩ`` = ``10000``, ``100n`` = ``0.1u``, ``4k7`` = ``4.7k``.
A unit on the filter value (``>=10kΩ``) must match the unit on the part's attribute; no unit matches any.
Several attributes can match one name (``volt`` also finds "Voltage - Rated"); a part passes if ANY does.
"""
from __future__ import annotations

import math
import re
from dataclasses import dataclass, field
from itertools import product
from typing import Any, Callable, Iterable, Optional, Sequence, Union

from .models import Part

# --------------------------------------------------------------------------------------
# Quantities: "10kΩ", "100 mW", "±1%", "4k7", "1.1 MHz"
# --------------------------------------------------------------------------------------
_PREFIX = {"p": 1e-12, "n": 1e-9, "u": 1e-6, "µ": 1e-6, "μ": 1e-6, "m": 1e-3, "k": 1e3, "K": 1e3, "M": 1e6, "G": 1e9, "T": 1e12}
_UNITS = {"ω": "Ω", "ohm": "Ω", "ohms": "Ω", "f": "F", "h": "H", "v": "V", "a": "A", "w": "W", "hz": "Hz",
          "s": "s", "vdc": "V", "vac": "V", "%": "%", "℃": "°C", "°c": "°C", "db": "dB", "ppm": "ppm", "ah": "Ah"}
_QTY = re.compile(r"^\s*(?P<sign>[±+\-−])?\s*(?P<num>\d+(?:[.,]\d+)?)\s*(?P<rest>[^\s~/\d,;]*)")
_RKM = re.compile(r"^\s*(\d+)([pnuµμmkKMGR])(\d+)\s*(\S*)\s*$")


@dataclass(frozen=True)
class Quantity:
    value: float  # in base units (ohm, farad, volt, ...)
    unit: str  # normalised unit symbol, "" when unitless

    def compatible(self, other: "Quantity") -> bool:
        return not self.unit or not other.unit or self.unit == other.unit


def _split_rest(rest: str) -> Optional[tuple[float, str]]:
    """Split the text after a number into (multiplier, unit): 'kΩ' -> (1e3, 'Ω'), 'W' -> (1, 'W'), 'k' -> (1e3, '')."""
    if rest == "":
        return 1.0, ""
    if rest in _PREFIX:  # a lone prefix letter is a multiplier ('10k', '4.7M'), never a unit
        return _PREFIX[rest], ""
    if rest.lower() in _UNITS:
        return 1.0, _UNITS[rest.lower()]
    if rest[0] in _PREFIX and rest[1:].lower() in _UNITS:
        return _PREFIX[rest[0]], _UNITS[rest[1:].lower()]
    return None


def parse_quantity(text: Any) -> Optional[Quantity]:
    """Parse ``"10kΩ"``-style text into a base-unit :class:`Quantity`; ``None`` if it is not a plain scalar.

    Ranges such as ``"-55℃~+155℃"`` are not scalars and return ``None``. A sign is applied only for
    ``-`` / ``−``; ``±`` and ``+`` mean "magnitude".
    """
    if text is None:
        return None
    s = str(text).strip()
    if not s:
        return None
    m = _RKM.match(s)
    if m:
        whole, p, frac, tail = m.groups()
        mult = 1.0 if p == "R" else _PREFIX.get(p)
        if mult is not None:
            unit = _UNITS.get(tail.lower(), "") if tail else ""
            if not tail or unit:
                return Quantity(float(f"{whole}.{frac}") * mult, unit)
    m = _QTY.match(s)
    if not m:
        return None
    after = s[m.end():].lstrip()
    if after.startswith("~") or (after[:1] in ("-", "−") and re.match(r"[-−]\s*\d", after)):
        return None  # a range, not a scalar
    parsed = _split_rest(m.group("rest"))
    if parsed is None:
        return None
    mult, unit = parsed
    value = float(m.group("num").replace(",", ".")) * mult
    if m.group("sign") in ("-", "−"):
        value = -value
    return Quantity(value, unit)


def _num_eq(a: float, b: float) -> bool:
    return math.isclose(a, b, rel_tol=1e-9, abs_tol=1e-15)


# --------------------------------------------------------------------------------------
# Conditions
# --------------------------------------------------------------------------------------
_OPS = (">=", "<=", "!=", ">", "<", "=")


@dataclass
class Cond:
    """One comparison. ``operands`` are alternatives (``a|b``) except for ``range`` (``[lo, hi]``)."""

    op: str  # "auto" | "eq" | "ne" | "gt" | "ge" | "lt" | "le" | "range" | "regex"
    operands: list[str]
    negate: bool = False

    @classmethod
    def parse(cls, expr: str) -> "Cond":
        expr = expr.strip()
        negate = False
        if expr.startswith("!") and not expr.startswith("!="):
            negate, expr = True, expr[1:].strip()
        if expr.startswith("~"):
            return cls("regex", [expr[1:]], negate)
        for sym, op in ((">=", "ge"), ("<=", "le"), ("!=", "ne"), (">", "gt"), ("<", "lt"), ("=", "eq")):
            if expr.startswith(sym):
                rest = expr[len(sym):].strip()
                return cls(op, [x.strip() for x in rest.split("|")] if op in ("eq", "ne") else [rest], negate)
        if ".." in expr:
            lo, hi = (x.strip() for x in expr.split("..", 1))
            return cls("range", [lo, hi], negate)
        return cls("auto", [x.strip() for x in expr.split("|") if x.strip()], negate)

    # -- evaluation against one text value -------------------------------------------
    def test(self, text: Optional[str]) -> bool:
        """Does ``text`` satisfy the condition (before ``negate``)?"""
        if text is None:
            return False
        t = str(text)
        if self.op == "regex":
            return re.search(self.operands[0], t, re.I) is not None
        if self.op in ("gt", "ge", "lt", "le"):
            return self._cmp(t, self.op, self.operands[0])
        if self.op == "range":
            return self._cmp(t, "ge", self.operands[0]) and self._cmp(t, "le", self.operands[1])
        if self.op == "ne":
            return not any(self._equal(t, o) for o in self.operands)
        if self.op == "eq":
            return any(self._equal(t, o) for o in self.operands)
        return any(self._contains_or_equal(t, o) for o in self.operands)  # auto

    @staticmethod
    def _cmp(text: str, op: str, operand: str) -> bool:
        a, b = parse_quantity(text), parse_quantity(operand)
        if a is None or b is None or not a.compatible(b):
            return False
        return {"gt": a.value > b.value and not _num_eq(a.value, b.value),
                "ge": a.value > b.value or _num_eq(a.value, b.value),
                "lt": a.value < b.value and not _num_eq(a.value, b.value),
                "le": a.value < b.value or _num_eq(a.value, b.value)}[op]

    @staticmethod
    def _equal(text: str, operand: str) -> bool:
        a, b = parse_quantity(text), parse_quantity(operand)
        if a is not None and b is not None and a.compatible(b) and _num_eq(a.value, b.value):
            return True
        return text.strip().casefold() == operand.strip().casefold()

    @staticmethod
    def _contains_or_equal(text: str, operand: str) -> bool:
        a, b = parse_quantity(text), parse_quantity(operand)
        if a is not None and b is not None and a.compatible(b) and _num_eq(a.value, b.value):
            return True
        return operand.casefold() in text.casefold()


_IMPERIAL_METRIC = re.compile(r"^\s*(\d{4})\s*\(\s*\d{4}\s*Metric\s*\)")


def package_matches(cond: "Cond", text: Optional[str]) -> bool:
    """``cond.test`` for package names, with one size rule: DigiKey writes chip sizes as "0201 (0603 Metric)" -
    imperial first, metric in brackets. A bare 4-digit filter like ``pkg:0603`` means the *imperial* size, so it must
    not match "0201 (0603 Metric)" (which is a different, smaller part). Longer filters still match the full text.
    """
    if text is not None and cond.op in ("auto", "eq", "ne") and all(re.fullmatch(r"\d{4}", o) for o in cond.operands):
        m = _IMPERIAL_METRIC.match(text)
        if m:
            text = m.group(1)
    return cond.test(text)


def _squash(s: str) -> str:
    return re.sub(r"[^0-9a-zäöüß]+", "", s.casefold())


# Names users type -> substrings of real attribute names (English for JLC/LCSC, German for Reichelt)
ATTR_ALIASES: dict[str, tuple[str, ...]] = {
    "res": ("resistance", "widerstand"), "resistance": ("resistance", "widerstand"),
    "cap": ("capacitance", "kapazit"), "capacitance": ("capacitance", "kapazit"),
    "ind": ("inductance", "induktivit"), "inductance": ("inductance", "induktivit"),
    "tol": ("tolerance", "toleranz"), "tolerance": ("tolerance", "toleranz"),
    "power": ("power", "leistung"), "pwr": ("power", "leistung"),
    "volt": ("voltage", "spannung"), "voltage": ("voltage", "spannung"), "v": ("voltage", "spannung"),
    "curr": ("current", "strom"), "current": ("current", "strom"),
    "freq": ("frequency", "frequenz"), "frequency": ("frequency", "frequenz"),
    "temp": ("temperature", "temperatur"), "temperature": ("temperature", "temperatur"),
    "type": ("type", "typ"), "channels": ("channel", "kanäle", "kanal"), "pins": ("pins", "pole", "polzahl"),
    "color": ("color", "colour", "farbe"), "colour": ("color", "colour", "farbe"),
}
_FIELD_KEYS = {
    "code": "code", "lcsc": "code", "jlc": "code", "art": "code", "sku": "code",
    "mpn": "mpn", "part": "mpn",
    "mfr": "manufacturer", "manufacturer": "manufacturer", "brand": "manufacturer", "hersteller": "manufacturer",
    "pkg": "package", "package": "package", "footprint": "package", "bauform": "package",
    "cat": "category", "category": "category",
    "desc": "description", "description": "description",
    "lib": "library", "library": "library",
    "stock": "stock", "price": "price",
}
_FLAGS = {"preferred", "instock", "rohs", "basic", "extended"}
_HAS = {"datasheet", "image", "picture", "photo", "price", "stock", "3dmodel"}


# --------------------------------------------------------------------------------------
# The filter
# --------------------------------------------------------------------------------------
@dataclass
class PartFilter:
    """A set of conditions a :class:`Part` must satisfy. Build it by hand or via :func:`parse_query`."""

    all_terms: list[str] = field(default_factory=list)  # every one must appear in the part text
    any_groups: list[list[str]] = field(default_factory=list)  # each group: at least one must appear
    exclude_terms: list[str] = field(default_factory=list)  # none may appear
    fields: list[tuple[str, Cond]] = field(default_factory=list)  # (field, cond), field in _FIELD_KEYS values
    attrs: list[tuple[str, Cond, bool]] = field(default_factory=list)  # (name or alias, cond, exact_name)
    flags: set[str] = field(default_factory=set)  # preferred instock rohs basic extended
    has: set[str] = field(default_factory=set)  # datasheet image price stock
    qty: int = 1  # quantity at which `price:` is evaluated

    # -- construction helpers ------------------------------------------------------------
    def add_field(self, name: str, expr: str) -> "PartFilter":
        self.fields.append((_FIELD_KEYS.get(name, name), Cond.parse(expr)))
        return self

    def add_attr(self, name: str, expr: str, exact_name: bool = False) -> "PartFilter":
        self.attrs.append((name, Cond.parse(expr), exact_name))
        return self

    @property
    def empty(self) -> bool:
        return not (self.all_terms or self.any_groups or self.exclude_terms or self.fields or self.attrs or self.flags or self.has)

    def merge(self, other: "PartFilter") -> "PartFilter":
        """Add all of ``other``'s conditions to this filter (in place)."""
        self.all_terms += [t for t in other.all_terms if t not in self.all_terms]
        self.any_groups += other.any_groups
        self.exclude_terms += [t for t in other.exclude_terms if t not in self.exclude_terms]
        self.fields += other.fields
        self.attrs += other.attrs
        self.flags |= other.flags
        self.has |= other.has
        return self

    # -- evaluation ---------------------------------------------------------------------
    def matches(self, part: Part, skip: Iterable[str] = ()) -> bool:
        """True if ``part`` passes. ``skip`` names check groups to ignore because the data is not loaded yet
        (``text attrs price stock datasheet image rohs mpn description library category package manufacturer``)."""
        skip = set(skip)
        if "text" not in skip and not self._text_ok(part):
            return False
        for name, cond in self.fields:
            if name in skip:
                continue
            ok = self._field_ok(part, name, cond)
            if ok == cond.negate:
                return False
        for name, cond, exact in self.attrs:
            if "attrs" in skip:
                continue
            ok = self._attr_ok(part, name, cond, exact)
            if ok == cond.negate:
                return False
        for flag in self.flags:
            if flag == "preferred" and "library" not in skip and not part.extra.get("preferred"):
                return False
            if flag == "instock" and "stock" not in skip and not (part.stock and part.stock.in_stock):
                return False
            if flag == "rohs" and "rohs" not in skip and not part.extra.get("rohs"):
                return False
            if flag in ("basic", "extended") and "library" not in skip and part.extra.get("library_type") != flag:
                return False
        for h in self.has:
            if h == "datasheet" and "datasheet" not in skip and not part.datasheet_url:
                return False
            if h in ("image", "picture", "photo") and "image" not in skip and not part.image_urls:
                return False
            if h == "price" and "price" not in skip and not part.price_breaks:
                return False
            if h == "stock" and "stock" not in skip and not (part.stock and part.stock.in_stock):
                return False
        return True

    def filter(self, parts: Iterable[Part], skip: Iterable[str] = ()) -> list[Part]:
        skip = tuple(skip)
        return [p for p in parts if self.matches(p, skip)]

    def _haystack(self, p: Part) -> tuple[str, str]:
        bits = [p.code, p.mpn, p.manufacturer, p.description, p.category, p.package]
        bits += [f"{k} {v}" for k, v in p.attributes.items()]
        hay = "\n".join(b for b in bits if b).casefold()
        return hay, _squash(hay)

    def _text_ok(self, p: Part) -> bool:
        hay, squashed = self._haystack(p)

        def has(term: str) -> bool:  # "lm358" must also find "LM 358" and "LM-358"
            t = term.casefold()
            return t in hay or (len(_squash(t)) >= 2 and _squash(t) in squashed)

        if any(not has(t) for t in self.all_terms):
            return False
        if any(not any(has(t) for t in g) for g in self.any_groups):
            return False
        return not any(has(t) for t in self.exclude_terms)

    def _field_ok(self, p: Part, name: str, cond: Cond) -> bool:
        if name == "price":
            v = p.price_for(self.qty)
            return v is not None and cond.test(repr(v))
        if name == "stock":
            q = p.stock.quantity if p.stock else None
            if q is None:  # vendor only says in/out of stock (Reichelt)
                return bool(p.stock and p.stock.in_stock) and cond.test("1") and cond.op in ("gt", "ge", "auto")
            return cond.test(str(q))
        if name == "library":
            return cond.test(p.extra.get("library_type"))
        value = {"code": p.code, "mpn": p.mpn, "manufacturer": p.manufacturer, "package": p.package,
                 "category": p.category, "description": p.description}.get(name)
        if name == "manufacturer" and cond.op == "auto":  # "(UNI-ROYAL(Uniroyal Elec))" -> substring is right
            return value is not None and any(o.casefold() in value.casefold() for o in cond.operands)
        if value is None and name == "package":
            value = next((v for k, v in p.attributes.items() if _squash(k) in ("bauform", "gehäuse", "gehause", "packagecase", "package", "case")), None)
        return package_matches(cond, value) if name == "package" else cond.test(value)

    @staticmethod
    def attr_name_matches(attr_name: str, wanted: str, exact: bool = False) -> bool:
        """Does a part's attribute name satisfy the name/alias used in a filter (``res`` -> "Resistance")?"""
        if exact:
            return _squash(attr_name) == _squash(wanted)
        needles = tuple(_squash(n) for n in ATTR_ALIASES.get(wanted.casefold(), (wanted,)))
        return any(n in _squash(attr_name) for n in needles)

    def _attr_ok(self, p: Part, name: str, cond: Cond, exact: bool) -> bool:
        hits = [v for k, v in p.attributes.items() if self.attr_name_matches(k, name, exact)]
        if not hits:
            return False
        return any(cond.test(v) for v in hits)

    def requires_detail(self, available: Iterable[str]) -> bool:
        """Does this filter need data that is not in ``available`` (check-group names)?"""
        need: set[str] = set()
        if self.all_terms or self.any_groups or self.exclude_terms:
            need.add("text")
        need |= {f for f, _ in self.fields}
        if self.attrs:
            need.add("attrs")
        need |= {"stock" for f in self.flags if f == "instock"} | {"rohs" for f in self.flags if f == "rohs"}
        need |= {"library" for f in self.flags if f in ("preferred", "basic", "extended")}
        need |= {"datasheet" if h == "datasheet" else "image" if h in ("image", "picture", "photo") else h for h in self.has}
        return bool(need - set(available))


def reject_unsupported(flt: PartFilter, vendor: str, *, flags: Iterable[str] = (), has: Iterable[str] = (), library: bool = False) -> None:
    """Raise ``ValueError`` if ``flt`` uses a filter ``vendor`` cannot evaluate."""
    bad = [f"is:{f}" for f in flt.flags if f in set(flags)] + [f"has:{h}" for h in flt.has if h in set(has)]
    if library and any(n == "library" for n, _ in flt.fields):
        bad.append("lib:")
    if bad:
        raise ValueError(f"{vendor} does not support {', '.join(sorted(bad))}")


# --------------------------------------------------------------------------------------
# Query parser
# --------------------------------------------------------------------------------------
_TOKEN = re.compile(r'''(?:-|@)?(?:"[^"]*"|[^\s:"]+)(?::(?:[<>=!~]*"[^"]*"|\S*))?|\S+''')


@dataclass
class ParsedQuery:
    """Result of :func:`parse_query`: what to ask the vendor, and what to check afterwards."""

    keywords: list[str]  # plain positive words/phrases sent to the vendor
    or_groups: list[list[str]]  # each group runs as separate searches and is merged
    filter: PartFilter

    def server_queries(self, max_searches: int = 8) -> list[str]:
        base = list(self.keywords)
        if not self.or_groups:
            return [" ".join(base)]
        combos = list(product(*self.or_groups))
        if len(combos) > max_searches:
            raise ValueError(f"query expands to {len(combos)} searches (max {max_searches}); reduce the OR groups")
        return [" ".join(base + list(c)) for c in combos]


def _unquote(s: str) -> str:
    """Strip quotes, also after an operator: ``"a b"`` -> ``a b``; ``="a b"`` -> ``=a b``."""
    m = re.fullmatch(r'([<>=!~]*)"(.*)"', s)
    return m.group(1) + m.group(2) if m else s


def parse_query(query: str, base: Optional[PartFilter] = None, qty: int = 1, strict: Optional[bool] = None) -> ParsedQuery:
    """Split a user query into vendor keywords plus a :class:`PartFilter` (see module docstring).

    Bare words are sent to the vendor's search. ``strict`` decides whether they must *also* appear in the
    part's text: ``True`` = every word, ``False`` = none, ``None`` (default) = only words containing a digit
    (``0603``, ``lm358``, ``10k``), because vendor search is fuzzy exactly where identifiers matter, while
    plain words like "resistor" or "amplifier" are left to the vendor's ranking. Quoted phrases are always
    enforced. Everything else becomes a filter.
    """
    def enforce(word: str) -> bool:
        return " " in word or strict is True or (strict is None and any(ch.isdigit() for ch in word))

    flt = base or PartFilter()
    flt.qty = qty
    keywords: list[str] = []
    or_groups: list[list[str]] = []
    last: Optional[str] = None  # "kw" | "grp": what the previous positive term was
    pending_or = False
    for tok in _TOKEN.findall(query or ""):
        if tok == "OR" and last is not None and not pending_or:
            pending_or = True
            continue
        neg = tok.startswith("-") and len(tok) > 1
        body = tok[1:] if neg else tok
        at = body.startswith("@")
        if at:
            body = body[1:]
        key, sep, val = body.partition(":")
        key_u = _unquote(key)
        lk = key_u.lower()
        if sep and val != "" and (at or lk in _FIELD_KEYS or lk in ATTR_ALIASES or lk in ("is", "has")):
            val = _unquote(val)
            if lk == "is" and not at and val.lower() in _FLAGS:
                flt.flags.add(val.lower())
            elif lk == "has" and not at and val.lower() in _HAS:
                flt.has.add(val.lower())
            else:
                expr = ("!" + val) if neg and not val.startswith("!") else val
                if lk in _FIELD_KEYS and not at:
                    flt.add_field(lk, expr)
                else:
                    flt.add_attr(key_u, expr, exact_name=at)
            continue
        text = _unquote(body)
        if not text:
            continue
        if neg:
            flt.exclude_terms.append(text)
            continue
        if pending_or:
            if last == "grp":
                or_groups[-1].append(text)
            else:
                or_groups.append([keywords.pop(), text])
                last = "grp"
            pending_or = False
        else:
            keywords.append(text)
            last = "kw"
    flt.all_terms.extend(k for k in keywords if enforce(k) and k not in flt.all_terms)
    flt.any_groups.extend(g for g in or_groups if strict is True or any(enforce(w) for w in g))
    return ParsedQuery(keywords=keywords, or_groups=or_groups, filter=flt)


# --------------------------------------------------------------------------------------
# Sorting and facets
# --------------------------------------------------------------------------------------
SORT_KEYS: dict[str, Callable[[Part, int], Any]] = {
    "price": lambda p, q: p.price_for(q),
    "stock": lambda p, q: (p.stock.quantity if p.stock and p.stock.quantity is not None else (1 if p.stock and p.stock.in_stock else None)),
    "mpn": lambda p, q: (p.mpn or p.code or "").casefold() or None,
    "code": lambda p, q: p.code.casefold(),
    "manufacturer": lambda p, q: (p.manufacturer or "").casefold() or None,
    "package": lambda p, q: (p.package or "").casefold() or None,
}
SORT_ALIASES = {"mfr": "manufacturer", "brand": "manufacturer", "pkg": "package", "name": "mpn", "lcsc": "code"}


def sort_parts(parts: Sequence[Part], key: Union[str, Callable[[Part], Any]], *, reverse: bool = False, qty: int = 1) -> list[Part]:
    """Sort by ``price | stock | mpn | code | manufacturer | package`` or a callable. Parts with no value go last."""
    if callable(key):
        fn = lambda p: key(p)  # noqa: E731
    else:
        k = SORT_ALIASES.get(key, key)
        if k not in SORT_KEYS:
            raise ValueError(f"unknown sort key {key!r}; use one of {sorted(SORT_KEYS)}")
        fn = lambda p: SORT_KEYS[k](p, qty)  # noqa: E731
    have = [(fn(p), p) for p in parts]
    present = sorted((x for x in have if x[0] is not None), key=lambda x: x[0], reverse=reverse)
    return [p for _, p in present] + [p for v, p in have if v is None]


def facets(parts: Iterable[Part], top: int = 15) -> dict[str, Any]:
    """Count what is in a result list so you know which filters make sense (manufacturers, packages, ...)."""
    from collections import Counter

    parts = list(parts)
    c: dict[str, Counter] = {k: Counter() for k in ("manufacturer", "package", "category", "library")}
    attr: dict[str, Counter] = {}
    for p in parts:
        for k, v in (("manufacturer", p.manufacturer), ("package", p.package), ("category", p.category),
                     ("library", p.extra.get("library_type"))):
            if v:
                c[k][v] += 1
        for k, v in p.attributes.items():
            if v:
                attr.setdefault(k, Counter())[v] += 1
    prices = [p.price_for(1) for p in parts if p.price_for(1) is not None]
    stocks = [p.stock.quantity for p in parts if p.stock and p.stock.quantity is not None]
    return {
        "total": len(parts),
        "manufacturers": c["manufacturer"].most_common(top),
        "packages": c["package"].most_common(top),
        "categories": c["category"].most_common(top),
        "libraries": c["library"].most_common(),
        "attributes": {k: v.most_common(top) for k, v in sorted(attr.items(), key=lambda kv: -sum(kv[1].values()))[:top]},
        "price_range": (min(prices), max(prices)) if prices else None,
        "stock_range": (min(stocks), max(stocks)) if stocks else None,
        "in_stock": sum(1 for p in parts if p.stock and p.stock.in_stock),
        "with_datasheet": sum(1 for p in parts if p.datasheet_url),
    }


def build_filter(where: Union[None, str, PartFilter], qty: int = 1) -> PartFilter:
    """Normalise a ``where=`` argument (expression string, :class:`PartFilter` or ``None``).

    In a ``where`` string bare words are *requirements* on the part text (there is no vendor search to trust).
    """
    if where is None:
        return PartFilter(qty=qty)
    if isinstance(where, PartFilter):
        where.qty = qty
        return where
    pq = parse_query(where, qty=qty, strict=True)
    return pq.filter
