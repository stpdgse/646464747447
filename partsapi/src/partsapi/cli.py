"""Command line: ``partsapi <jlc|lcsc|reichelt> <command> <arg> [options]`` - prints JSON."""
from __future__ import annotations

import argparse
import dataclasses
import json
import re
import sys
from pathlib import Path
from typing import Any

from . import digikey, jlc, lcsc, mouser, parts, reichelt
from .errors import PartsApiError
from .filters import ATTR_ALIASES

VENDORS = {"jlc": jlc, "lcsc": lcsc, "reichelt": reichelt, "digikey": digikey, "mouser": mouser}
COMMANDS = ["findpart", "facets", "getpart", "getpartprice", "getprice", "getstock", "getattributes",
            "getpics", "getpic", "getdatasheet", "get3dmodel", "getsymbol", "getfootprint"]


def _plain(o: Any) -> Any:
    if dataclasses.is_dataclass(o) and not isinstance(o, type):
        if hasattr(o, "to_dict"):
            return o.to_dict()
        d = dataclasses.asdict(o)
        d.pop("data", None)
        return d
    if isinstance(o, (list, tuple)):
        return [_plain(x) for x in o]
    if isinstance(o, dict):
        return {k: _plain(v) for k, v in o.items()}
    if isinstance(o, Path):
        return str(o)
    return o


def build_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(prog="partsapi", description=__doc__)
    sub = p.add_subparsers(dest="vendor", required=True)
    for name in VENDORS:
        vp = sub.add_parser(name)
        cp = vp.add_subparsers(dest="command", required=True)
        for cmd in COMMANDS:
            c = cp.add_parser(cmd)
            c.add_argument("query", help="search text (findpart) or part number / URL")
            c.add_argument("--qty", type=int, help="quantity for price lookups")
            c.add_argument("--limit", type=int, default=10, help="max search results")
            c.add_argument("--out", help="save file/directory (pics, datasheets, 3D models)")
            c.add_argument("--fmt", default="step", choices=["step", "obj"], help="3D model format")
            c.add_argument("--library", choices=["basic", "extended"], help="jlc/lcsc: JLCPCB library class")
            f = c.add_argument_group("filters (findpart/facets; combine freely, all also work inside the query)")
            f.add_argument("--where", action="append", default=[], help="filter expression, e.g. 'res:>=10k tol:<=1%% -thick'")
            f.add_argument("--mfr", action="append", default=[], help="manufacturer contains (repeatable = OR)")
            f.add_argument("--package", action="append", default=[], help="EXACT package name (repeatable = OR); for substring use --where \"pkg:SOIC\"")
            f.add_argument("--category", help="category contains, e.g. resistor")
            f.add_argument("--attr", action="append", default=[], metavar="NAME=EXPR", help="attribute filter, e.g. resistance='>=10k'")
            f.add_argument("--exclude", action="append", default=[], help="drop parts containing this word (repeatable)")
            f.add_argument("--in-stock", action="store_true")
            f.add_argument("--min-stock", type=int)
            f.add_argument("--min-price", type=float, help="unit price at --qty, lower bound")
            f.add_argument("--max-price", type=float, help="unit price at --qty, upper bound")
            f.add_argument("--preferred", action="store_true", help="jlc: preferred parts only")
            f.add_argument("--rohs", action="store_true")
            f.add_argument("--has-datasheet", action="store_true")
            f.add_argument("--has-image", action="store_true")
            f.add_argument("--sort", choices=["price", "stock", "mpn", "code", "manufacturer", "package"])
            f.add_argument("--desc", action="store_true", help="reverse the sort order")
            f.add_argument("--max-pages", type=int, help="result pages to read when filtering client-side")
            f.add_argument("--strict", action="store_true", help="every search word must appear in the part text")
            c.add_argument("--table", action="store_true", help="findpart: compact table instead of JSON")
            c.add_argument("--currency", help="lcsc price currency, e.g. EUR")
    cmp_ = sub.add_parser("prices", help="unit price per vendor for one MPN / part number")
    cmp_.add_argument("query")
    cmp_.add_argument("--qty", type=int, default=1)
    return p


def build_where(args: argparse.Namespace) -> str:
    """Turn the convenience flags into one filter expression."""
    q = lambda v: f'"{v}"' if re.search(r"[\s|]", v) and not re.search(r"\|", v) else v  # noqa: E731
    bits: list[str] = list(args.where)
    if args.mfr:
        bits.append("mfr:" + "|".join(args.mfr))
    if args.package:
        bits.append("pkg:=" + "|".join(args.package))
    if args.category:
        bits.append(f"cat:{q(args.category)}")
    for a in args.attr:
        name, _, expr = a.partition("=")
        if not expr:
            raise SystemExit(f"--attr needs NAME=EXPR, got {a!r}")
        bits.append(f"@{q(name)}:{expr}" if name.lower() not in ATTR_ALIASES else f"{name}:{expr}")
    bits += [f"-{q(x)}" for x in args.exclude]
    if args.in_stock:
        bits.append("is:instock")
    if args.min_stock is not None:
        bits.append(f"stock:>={args.min_stock}")
    if args.min_price is not None:
        bits.append(f"price:>={args.min_price}")
    if args.max_price is not None:
        bits.append(f"price:<={args.max_price}")
    for flag, on in (("preferred", args.preferred), ("rohs", args.rohs)):
        if on:
            bits.append(f"is:{flag}")
    if args.has_datasheet:
        bits.append("has:datasheet")
    if args.has_image:
        bits.append("has:image")
    return " ".join(bits)


def table(parts: list) -> str:
    cols = ["code", "mpn", "manufacturer", "package", "price", "stock"]
    rows = [[p.code, p.mpn or "", p.manufacturer or "", p.package or "",
             "" if p.price_for(1) is None else f"{p.price_for(1):g}",
             "" if not p.stock else (str(p.stock.quantity) if p.stock.quantity is not None else ("yes" if p.stock.in_stock else "no"))]
            for p in parts]
    widths = [max(len(c), *(len(r[i][:34]) for r in rows)) if rows else len(c) for i, c in enumerate(cols)]
    line = lambda r: "  ".join(str(v)[:34].ljust(w) for v, w in zip(r, widths))  # noqa: E731
    return "\n".join([line(cols), line(["-" * w for w in widths])] + [line(r) for r in rows])


def run(args: argparse.Namespace) -> Any:
    if args.vendor == "prices":
        return parts.prices(args.query, args.qty)
    mod = VENDORS[args.vendor]
    cmd, q = args.command, args.query
    if args.currency:
        if args.vendor != "lcsc":
            raise SystemExit("--currency only applies to lcsc")
        lcsc.set_currency(args.currency)
    if cmd == "facets" and args.vendor == "jlc":  # server-side facets take scope, not a filter expression
        return jlc.facets(q, category=args.category, library=args.library, brands=args.mfr, packages=args.package)
    if cmd in ("findpart", "facets"):
        kw: dict[str, Any] = {}
        where = build_where(args)
        if where:
            kw["where"] = where
        if args.library:
            if args.vendor not in ("jlc", "lcsc"):
                raise SystemExit("--library only applies to jlc and lcsc")
            where = (where + " " if where else "") + f"lib:{args.library}"
            kw["where"] = where
        if cmd == "findpart":
            kw.update(limit=args.limit, sort=args.sort, reverse=args.desc, qty=args.qty or 1)
        if args.max_pages and args.vendor != "reichelt":
            kw["max_pages"] = args.max_pages
        if args.strict:
            kw["strict"] = True
        if cmd == "findpart" and args.vendor == "reichelt":
            kw.pop("max_pages", None)
        res = getattr(mod, cmd)(q, **kw)
        return table(res) if (args.table and cmd == "findpart") else res
    if cmd in ("getpartprice", "getprice"):
        return mod.getpartprice(q, args.qty)
    fn = getattr(mod, cmd, None)
    if fn is None:
        raise SystemExit(f"{args.vendor} has no command {cmd!r}")
    if cmd == "get3dmodel":
        out = Path(args.out or ".")
        m = fn(q, args.fmt, out)
        return {**_plain(m), "saved_to": str(out / m.filename if out.is_dir() else out)}
    if cmd in ("getpic", "getdatasheet", "getsymbol", "getfootprint"):
        res = fn(q, args.out or ".")
        return {"saved_to": res}
    return fn(q)


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    try:
        result = run(args)
    except PartsApiError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 1
    except ValueError as exc:  # unsupported filter, bad argument
        print(f"error: {exc}", file=sys.stderr)
        return 2
    if isinstance(result, str):  # --table
        print(result)
        return 0
    try:
        json.dump(_plain(result), sys.stdout, indent=2, ensure_ascii=False, default=str)
        sys.stdout.write("\n")
    except BrokenPipeError:  # e.g. `| head`
        sys.stderr.close()
    return 0


if __name__ == "__main__":
    sys.exit(main())
