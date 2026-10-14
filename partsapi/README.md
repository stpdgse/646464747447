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
| `findpart(query, limit=, where=, sort=, qty=)` | yes (+ `library`, `preferred`, `in_stock_only`, `min_stock`, `brands`, `packages`, `attributes`, `category`, `subcategory`, `has_datasheet`, `has_image`, `max_pages`, `page`) | yes (+ `max_pages`, `max_fetch`) | yes (first result page, + `max_fetch`) |
| `facets(query)` | whole-catalogue counts | over results | over results |
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

## Search terms and filters

`findpart` understands a small query language. Put it in the query, in `where=`, or mix both; it works the
same on `jlc`, `lcsc`, `reichelt` and `parts.findpart` (all vendors at once).

```python
jlc.findpart('cat:resistor res:10k..100k tol:<=1% pkg:=0603 lib:basic is:instock', sort="price", limit=10)
jlc.findpart('lm358 OR lm324 mfr:ti|onsemi -automotive stock:>=1000 price:<0.2', qty=100, sort="price")
lcsc.findpart('lm358 pkg:soic is:instock has:datasheet', sort="price")
reichelt.findpart('LM358 -dip price:<=0.2 volt:>=30V', sort="price")
jlc.findpart("resistor", where="res:>=10k", library="basic", min_stock=1000)   # keyword arguments work too
parts.findpart('lm358 mfr:ti', vendors=("jlc", "lcsc", "reichelt"), sort="price")
```

| Syntax | Meaning |
|---|---|
| `resistor 0603` | search words (AND). Words containing a digit (`0603`, `lm358`) must also appear in the part text, because vendor search is fuzzy exactly there; plain words are left to the vendor's ranking. `strict=True` requires all words, `strict=False` none |
| `"thick film"` | exact phrase (always enforced) |
| `lm358 OR lm324` | alternatives: one search per alternative, merged (max 8 combinations) |
| `-automotive`, `-"low power"` | exclude parts whose text contains it |
| `mfr:ti\|onsemi`, `mfr:=TI`, `mfr:!yageo` | manufacturer contains / equals / not (`\|` = OR) |
| `pkg:0603`, `pkg:=SOIC-8`, `-pkg:0402` | package contains / equals / not |
| `cat:resistor`, `mpn:LM358`, `code:C25`, `desc:"dual op"` | category, manufacturer part number, vendor code, description |
| `lib:basic`, `lib:extended` (= `is:basic`, `is:extended`) | JLCPCB library class (jlc, lcsc) |
| `is:preferred`, `is:instock`, `is:rohs` | flags (`is:preferred`: jlc only) |
| `has:datasheet`, `has:image`, `has:price`, `has:stock` | data present |
| `stock:>=1000`, `price:<0.01` | numeric; `price` is the unit price at `qty=` (so `qty=100` compares the 100-piece price) |
| `res:>=10k`, `cap:100n`, `tol:<=1%`, `power:>=0.25W`, `volt:>=50V`, `curr:`, `freq:`, `temp:`, `ind:` | attribute filters; aliases also know the German names Reichelt uses |
| `@"Number of Pins":>=8` | any attribute by its exact (case-insensitive) name |
| `a..b`, `>=`, `<=`, `>`, `<`, `=`, `!=`, `~regex` | operators |

Numbers carry SI prefixes and units: `10k` = `10kΩ` = `10000`, `100n` = `0.1u` = `100nF`, `4k7` = 4700. A unit
on the filter value must match the attribute's unit (`>=10k` never matches `10nF`); no unit matches any. If a
name matches several attributes (`volt` hits "Voltage Rating" and "Voltage Rated"), a part passes if any one does.
Values that are not a single number (ranges like `-55℃~+155℃`, `3,0 ... 36,0 VDC`) never satisfy a numeric condition.
Text matching ignores spaces and dashes, so `lm358` finds Reichelt's `LM 358 DIP`.

**Sorting:** `sort="price" | "stock" | "mpn" | "code" | "manufacturer" | "package"` (or any callable), `reverse=True`.
Parts without a value go last. Sorting is over the candidates that were read (see below), not the whole catalogue.

**Facets** show which filter values exist before you pick one: `jlc.facets("", category="capacitor")` returns the
whole-catalogue counts of categories, brands, packages, every attribute's values, and basic / preferred /
with-datasheet totals. `lcsc.facets(query)` and `reichelt.facets(query)` count the search results instead. For any
list you already have: `partsapi.filters.facets(parts)`.

What each vendor does with a filter:

| | JLCPCB | LCSC | Reichelt |
|---|---|---|---|
| library, preferred, in-stock, min stock, datasheet/image, exact package/brand | **server-side** | - | - |
| numeric attributes (`res:10k..100k`) | **server-side** when a category is known (`cat:resistor` or `category=`): the real catalogue values are looked up and the matching ones sent to JLCPCB. Ambiguous names or negations are checked client-side | after fetch | on result cards |
| everything else | client-side while paging up to `max_pages` x 72 | manufacturer / MPN / package / `lib:` on the cheap search hits, the rest after fetching each survivor (`max_fetch`) | on cards; stock / datasheet / MPN / RoHS fetch the product pages of survivors (`max_fetch`) |

Without a category, an attribute-only JLCPCB query (`res:>=10k` and nothing else) can only filter the pages it
reads; give it `cat:` or `category=` / `subcategory=` and it filters the whole catalogue. `limit` goes up to 5000 on JLCPCB.
Unsupported combinations raise `ValueError` (`is:preferred` on LCSC/Reichelt, `lib:` on Reichelt, filters with no search
word on LCSC/Reichelt); `parts.findpart` turns those into a warning and an empty list for that vendor.

CLI:

```bash
partsapi jlc findpart resistor --category resistor --attr resistance=10k..100k --attr tolerance='<=1%' \
    --package 0603 --library basic --in-stock --max-price 0.002 --sort price --limit 5 --table
partsapi lcsc findpart lm358 --mfr 'texas|ti' --where 'pkg:soic' --sort price --table
partsapi reichelt findpart LM358 --exclude dip --max-price 0.2 --table
partsapi jlc facets "" --category capacitor
```

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
pytest                               # 177 offline tests against recorded fixtures
PARTSAPI_LIVE=1 pytest -m live       # hits the real sites
```
