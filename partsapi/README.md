# partsapi

One Python API for **JLCPCB parts**, **LCSC** and **Reichelt**: search, price, stock, pictures, datasheets and
3D models, with the same function names on every vendor.

```python
from partsapi import jlc, lcsc, reichelt

jlc.findpart("10k 0603 resistor", library="basic", limit=5)   # -> [Part, ...]
jlc.getpartprice("C25804", qty=1000)                          # -> 0.0043  (USD, unit price at qty)
jlc.getpartprice("C25804")                                    # -> [PriceBreak, ...] full ladder
jlc.getstock("C25804")                                        # -> Stock(quantity=31020621, ...)
jlc.getpic("C25804", save_to="out/")                          # -> Path  (bytes if save_to omitted)
jlc.get3dmodel("C2913204", fmt="step", save_to="out/")        # -> Model3D (.data = file bytes)
jlc.getpart("C25804")                                         # everything, as a Part

lcsc.getpartprice("C7950", qty=100)
lcsc.set_currency("EUR")                                      # LCSC prices in another currency

reichelt.getprice("LM358DT")                                  # 0.15  (EUR, gross incl. VAT)
reichelt.getstock("LM358DT")                                  # in_stock + delivery estimate
reichelt.getdatasheet("LM358DT", save_to="out/")
reichelt.get3dmodel("LM358DT")                                # resolved via the MPN, see below

from partsapi import parts
parts.prices("LM358DT", qty=10)                               # one row per vendor, not ranked
```

Install: `pip install -e partsapi` (needs `requests`, `beautifulsoup4`, `lxml`).
CLI: `python -m partsapi jlc getpartprice C25804 --qty 1000`, `python -m partsapi reichelt getstock LM358DT`,
`python -m partsapi lcsc get3dmodel C2913204 --fmt obj --out out/`, `python -m partsapi prices LM358DT --qty 10`.

## Functions

| | `jlc` | `lcsc` | `reichelt` |
|---|---|---|---|
| `findpart(query, limit=)` | yes (+ `library`, `preferred`, `in_stock_only`, `min_stock`, `brands`, `packages`, `attributes`, `page`) | yes | yes (first result page) |
| `getpart(code)` - everything, as `Part` | yes | yes | yes (code or product URL) |
| `getpartprice(code, qty=None)` / `getprice` | ladder, or unit price at `qty` | same | single unit price |
| `getstock(code)` | quantity | quantity + shipping split | in/out of stock + delivery text |
| `getattributes(code)` | yes | yes | yes |
| `getpics(code)` / `getpic(code, save_to=)` | yes | yes | yes |
| `getdatasheet(code, save_to=)` | yes | yes | yes |
| `get3dmodel(code, fmt="step"\|"obj", save_to=)` | yes | yes | yes (via MPN) |
| `getsymbol` / `getfootprint` (SVG) | - | yes | - |

Part numbers: JLCPCB and LCSC share the `C12345` numbering. Reichelt uses its own article numbers (`LM358DT`).
Every function also exists as a method on `JLC()`, `LCSC()`, `Reichelt()` if you want your own `Http` (timeouts,
proxy, rate limit, cache TTL).

## How it works, and what to know

* **Unofficial.** None of these sites publishes this API. JLCPCB and LCSC are called at the JSON endpoints their
  own web pages use; Reichelt is parsed from HTML (schema.org microdata). Any of them can change without notice -
  the parsers raise `ParseError` instead of returning wrong data, and `tests/test_live.py` tells you what broke.
  Check each site's terms before heavy use. Requests are throttled (0.25 s per host, 0.5 s for Reichelt),
  retried with backoff on 429/5xx, and cached for 5 minutes.
* **Prices.** JLCPCB/LCSC: net, USD unless you switch LCSC currency. **Reichelt: EUR gross including German VAT**
  (`PriceBreak.incl_vat=True`, `vat_rate=0.19`). Reichelt shows one unit price, so `qty` does not change it.
  `parts.prices()` lists vendors side by side and deliberately does not rank across currencies and tax bases.
* **Stock.** Reichelt does not publish unit counts: `Stock.quantity` is `None`, `in_stock` and `delivery` are set.
* **LCSC keyword search.** LCSC's search endpoints are behind bot protection (403), so `lcsc.findpart` searches the
  same catalogue through EasyEDA and then fetches each hit from LCSC. A bare C-number skips the search.
* **JLCPCB picture/datasheet links are signed and expire after ~30 minutes.** Download immediately (the `getpic`
  and `getdatasheet` functions do), don't store the URLs. LCSC and Reichelt image URLs are stable.
* **Reichelt search** returns the first result page (about 10-30 articles); the shop ignores page parameters in
  server-rendered HTML. Reichelt page text and attribute names are German.
* **3D models** come from the JLCEDA/EasyEDA library (STEP and OBJ) for any LCSC/JLCPCB part that has one;
  otherwise `NoModelError`. Reichelt hosts none, so `reichelt.get3dmodel` looks up the article's manufacturer
  part number in that library and accepts only an **exact MPN match** - `Model3D.source_code` tells you which
  LCSC part it came from. Package-compatible is not the same as identical: check before putting it on a board.
* **Attribution.** The EasyEDA library is copyrighted by JLCEDA. If you redistribute models, symbols or footprints,
  credit "JLCEDA/EasyEDA Official Library" with links to https://lceda.cn/ and https://easyeda.com
  (`Model3D.attribution` carries this text).

## Errors

`NotFoundError` (no such part/asset), `NoModelError` (a `NotFoundError`), `BlockedError` (403/bot protection,
or 429 after retries), `ParseError` (site layout changed), all subclasses of `PartsApiError`.

## Tests

```bash
pytest                               # 51 offline tests against recorded fixtures
PARTSAPI_LIVE=1 pytest -m live       # hits the real sites
```
