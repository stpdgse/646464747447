"""Command line: ``partsapi <jlc|lcsc|reichelt> <command> <arg> [options]`` - prints JSON."""
from __future__ import annotations

import argparse
import dataclasses
import json
import sys
from pathlib import Path
from typing import Any

from . import jlc, lcsc, parts, reichelt
from .errors import PartsApiError

VENDORS = {"jlc": jlc, "lcsc": lcsc, "reichelt": reichelt}
COMMANDS = ["findpart", "getpart", "getpartprice", "getprice", "getstock", "getattributes",
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
            c.add_argument("--library", choices=["basic", "extended"], help="jlc findpart filter")
            c.add_argument("--currency", help="lcsc price currency, e.g. EUR")
    cmp_ = sub.add_parser("prices", help="unit price per vendor for one MPN / part number")
    cmp_.add_argument("query")
    cmp_.add_argument("--qty", type=int, default=1)
    return p


def run(args: argparse.Namespace) -> Any:
    if args.vendor == "prices":
        return parts.prices(args.query, args.qty)
    mod = VENDORS[args.vendor]
    cmd, q = args.command, args.query
    if args.currency:
        if args.vendor != "lcsc":
            raise SystemExit("--currency only applies to lcsc")
        lcsc.set_currency(args.currency)
    if cmd == "findpart":
        kw: dict[str, Any] = {"limit": args.limit}
        if args.library:
            if args.vendor != "jlc":
                raise SystemExit("--library only applies to jlc")
            kw["library"] = args.library
        return mod.findpart(q, **kw)
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
    json.dump(_plain(result), sys.stdout, indent=2, ensure_ascii=False, default=str)
    sys.stdout.write("\n")
    return 0


if __name__ == "__main__":
    sys.exit(main())
