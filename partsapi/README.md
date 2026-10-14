# partsapi

One Python API for **JLCPCB parts**, **LCSC**, **Reichelt**, **DigiKey**, **Mouser**, **TME**, **Adafruit** and
**SparkFun**: search, price, stock, pictures, datasheets and 3D models, with the same function names and the same filter
language on every vendor. JLCPCB, LCSC, Reichelt, Adafruit and SparkFun need no key; DigiKey, Mouser and TME use their
official APIs and need a free one (see [DigiKey and Mouser](#digikey-and-mouser), [TME](#tme) and
[Adafruit and SparkFun](#adafruit-and-sparkfun)).

```python
from partsapi import jlc, lcsc, reichelt, digikey, mouser, tme, adafruit, sparkfun

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

digikey.getpartprice("296-1395-1-ND", qty=100)                # needs DIGIKEY_CLIENT_ID / _SECRET
mouser.findpart("lm358 is:instock -dip", sort="price")         # needs MOUSER_API_KEY
tme.findpart("resistor res:10k..100k tol:<=1% pkg:0603 is:instock", sort="price")   # needs TME_TOKEN / TME_SECRET
adafruit.findpart("feather rp2040 is:instock", sort="price")   # no key; searches a cached copy of the catalogue
sparkfun.findpart('sensor cat:"Environmental Sensors" @"Sensor Type":humidity')   # no key

from partsapi import parts
parts.prices("LM358DT", qty=10)                               # one row per vendor, not ranked
```

Install: `pip install -e partsapi` (needs `requests`, `beautifulsoup4`, `lxml`).
CLI: `python -m partsapi digikey findpart lm358 --mfr texas --in-stock --sort price --table`,
`python -m partsapi jlc getpartprice C25804 --qty 1000`, `python -m partsapi reichelt getstock LM358DT`,
`python -m partsapi lcsc get3dmodel C2913204 --fmt obj --out out/`, `python -m partsapi prices LM358DT --qty 10`.

## Functions

| | `jlc` | `lcsc` | `reichelt` | `digikey` | `mouser` |
|---|---|---|---|---|---|
| `findpart(query, limit=, where=, sort=, qty=)` | yes (+ `library`, `preferred`, `in_stock_only`, `min_stock`, `brands`, `packages`, `attributes`, `category`, `subcategory`, `has_datasheet`, `has_image`, `max_pages`, `page`) | yes (+ `max_pages`, `max_fetch`) | yes (first result page, + `max_fetch`) | yes (+ `max_pages`, `extra`) | yes (+ `max_pages`) |
| `facets(query)` | whole-catalogue counts | over results | over results | server-side counts | over results |
| `getpart(code)` - everything, as `Part` | yes | yes | yes (code or product URL) | yes (DigiKey number or MPN, `manufacturer=`) | yes (Mouser number or MPN) |
| `getpartprice(code, qty=None)` / `getprice` | ladder, or unit price at `qty` | same | single unit price | same, per package type | same |
| `getstock(code)` | quantity | quantity + shipping split | in/out of stock + delivery text | quantity + per-package split | quantity + on-order dates |
| `getattributes(code)` | yes | yes | yes | yes (all parameters) | yes |
| `getpics(code)` / `getpic(code, save_to=)` | yes | yes | yes | yes | yes |
| `getdatasheet(code, save_to=)` | yes | yes | yes | yes | yes |
| `get3dmodel(code, fmt="step"\|"obj", save_to=)` | yes | yes | yes (via MPN) | yes (via MPN) | yes (via MPN) |
| `getsymbol` / `getfootprint` (SVG) | - | yes | - | - | - |

Part numbers: JLCPCB and LCSC share the `C12345` numbering. Reichelt uses its own article numbers (`LM358DT`).
DigiKey numbers are per package type (`296-1395-1-ND` = cut tape, `-2-ND` = reel); an MPN works too. Mouser numbers look
like `595-LM358DR`; an exact MPN works too.
Every function also exists as a method on `JLC()`, `LCSC()`, `Reichelt()`, `DigiKey()`, `Mouser()` if you want your own
`Http` (timeouts, proxy, rate limit, cache TTL) or explicit credentials.

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

| | JLCPCB | LCSC | Reichelt | DigiKey | Mouser |
|---|---|---|---|---|---|
| in-stock, RoHS, datasheet / image | **server-side** (RoHS: client) | - | - | **server-side** | in-stock + RoHS server-side |
| library, preferred, exact package / brand | **server-side** | `lib:` on search hits | - | n/a | n/a |
| min stock, manufacturer, category | min stock + exact brand | after fetch | on cards | **server-side** (names resolved to DigiKey ids) | manufacturer client-side; no category data to filter on |
| numeric attributes (`res:10k..100k`) | **server-side** when a category is known: real catalogue values are looked up and the matching ones sent | after fetch | on result cards | **server-side** when one *leaf* category is selected (`cat:"Chip Resistor - Surface Mount"`, not `cat:resistors`); else client-side | **not available** (Mouser's API returns no attributes) |
| sorting | client-side | client-side | client-side | **server-side** for price (qty 1) / stock / mpn / code / manufacturer | client-side |
| everything else | client-side while paging up to `max_pages` x 72 | on the cheap search hits, the rest after fetching each survivor (`max_fetch`) | on cards; stock / datasheet / MPN / RoHS fetch the product pages of survivors (`max_fetch`) | client-side while paging, 50 per call | client-side while paging, 50 per call |

Without a category, an attribute-only JLCPCB query (`res:>=10k` and nothing else) can only filter the pages it
reads; give it `cat:` or `category=` / `subcategory=` and it filters the whole catalogue. `limit` goes up to 5000 on JLCPCB.
Unsupported combinations raise `ValueError` (`is:preferred` anywhere but JLCPCB, `lib:` on Reichelt / DigiKey / Mouser,
`has:3dmodel` anywhere but DigiKey, filters with no search word on LCSC / Reichelt / DigiKey / Mouser); `parts.findpart`
turns those into a warning and an empty list for that vendor. `mfr:` is a *contains* match on the vendor's spelling
("Texas Instruments", not "TI"): use `mfr:texas`, or `mfr:=name` for equality.

CLI:

```bash
partsapi jlc findpart resistor --category resistor --attr resistance=10k..100k --attr tolerance='<=1%' \
    --package 0603 --library basic --in-stock --max-price 0.002 --sort price --limit 5 --table
partsapi lcsc findpart lm358 --mfr 'texas|ti' --where 'pkg:soic' --sort price --table
partsapi reichelt findpart LM358 --exclude dip --max-price 0.2 --table
partsapi jlc facets "" --category capacitor
```

## DigiKey and Mouser

Both use the vendors' **official** APIs, which need a free key. The keys are read from the environment (or passed to
`DigiKey(...)` / `Mouser(...)`), never written anywhere, and are redacted from error messages.

| | DigiKey | Mouser |
|---|---|---|
| Get access | an app at https://developer.digikey.com/ with **Product Information V4** enabled | a Search API key at https://www.mouser.com/api-hub/ |
| Environment | `DIGIKEY_CLIENT_ID`, `DIGIKEY_CLIENT_SECRET` (or `DK_CLIENT_ID`, `DK_CLIENT_SECRET`); optional `DIGIKEY_SITE=US`, `DIGIKEY_LANGUAGE=en`, `DIGIKEY_CURRENCY=USD`, `DIGIKEY_SANDBOX=1` | `MOUSER_API_KEY` |
| Auth | two-legged OAuth2 (client credentials), token cached and refreshed automatically | key in the query string |
| Prices | standard pricing in the site/currency you configured (net of tax for the US site) | as Mouser formats them for your account; `PriceBreak.currency` is Mouser's code |
| Budget | one call per page of 50 plus occasional lookups (manufacturer/category lists are cached) | one call per page of 50; calls are spaced 2 s apart (Mouser allows about 30/min) |

`parts.default_vendors()` includes DigiKey, Mouser and TME only when configured, so `parts.findpart(...)` and `parts.prices(...)` keep
working without keys; name them in `vendors=` to require them (a missing key then becomes a warning / an error row).

* **Verification status.** **DigiKey** and **Mouser** were run against the real services with real accounts (search, filters,
  sorting, details, pricing, stock, pictures, datasheets - see `tests/test_live.py`), and several details of this README
  come from what the live responses showed. The offline fixtures for both are schema-derived
  (`tests/fixtures/*_synthetic.json`), not recordings. Live tests skip themselves without keys.
* **DigiKey package types.** One DigiKey product has several orderable variations (cut tape, reel, Digi-Reel). `Part.code`
  is the variation you asked for (or the first), and `Part.extra["variations"]` lists all with their own stock, minimum
  order quantity and price ladder. `getprice("LM358DR", qty)` picks the cheapest variation orderable at `qty`.
* **DigiKey parametric filters need a leaf category.** DigiKey only returns parameter / value ids for leaf categories, so
  `res:10k..100k tol:<=1% pkg:0603` is sent to DigiKey (and searches the whole catalogue) only with something like
  `cat:"Chip Resistor - Surface Mount"`; with `cat:resistors` or no category those conditions are applied client-side
  to the pages read (`max_pages` x 50), which can miss matches when results are sorted by price. `cat:` prefers an exact
  category name before substring matches. DigiKey needs at least one search word.
* **Package sizes.** DigiKey writes chip sizes as `0201 (0603 Metric)`: imperial first. A bare `pkg:0603` means imperial
  0603 (`0603 (1608 Metric)`) and does *not* match `0201 (0603 Metric)`; use `pkg:"1608 Metric"` for metric names.
* **Mouser.** The API can only filter by RoHS and in-stock server-side; manufacturer, price, stock and text are applied while
  paging (`max_pages`, default 3 x 50 results). It returns **no technical attributes and no package size** (only packaging
  options), so `res:` / `volt:` / `pkg:` / `@"Name":` filters raise `ValueError`; put the value in the search words
  (`resistor 0603 10k` is matched against Mouser's description) or use DigiKey / TME / LCSC. Language and currency follow
  your Mouser account (a German account gets German text and EUR). Some parts have no datasheet link at all. Mouser's
  manufacturer-filtered V2 search is not used because its `pageNumber` base is unspecified.
* **3D models, pictures, datasheets.** Neither vendor serves 3D models, so `get3dmodel` resolves the manufacturer part
  number in the LCSC/EasyEDA library, exact match only (same as Reichelt). Image and datasheet links are downloaded with a
  browser User-Agent; some manufacturer-hosted datasheets still refuse scripted downloads.

## Adafruit and SparkFun

Two hobby-electronics shops, both **keyless** and both read through interfaces the shops publish for their own use. They
sell boards, breakouts and modules under their own part numbers, so they rarely match a manufacturer part number: that is
why `parts.findpart()` / `parts.prices()` use them only when you name them (`vendors=("adafruit", "sparkfun")`).

| | `adafruit` | `sparkfun` |
|---|---|---|
| Source | public JSON product API (`/api/products`, `/api/product/<id>`) | public storefront GraphQL (`/graphql`, Magento) |
| How search works | the catalogue (~8 MB, 5,600 products) is downloaded **once**, cached on disk for 24 h (`PARTSAPI_CACHE_DIR`, `off` disables) and searched locally; the site search is never called | queries go to SparkFun (relevance order); ~1 request per page of 100, 1 s apart |
| Codes | product id (`4884`, `ADA4884` or the URL) | SKU (`SEN-19833`) |
| Price | USD, with quantity tiers | USD, with quantity tiers |
| Stock | "in stock" without a count, or a number; discontinued products are hidden unless `include_discontinued=True` | in / out of stock only (the API gives no counts) |
| Server-side filters | - (local) | search words, `cat:` (category ids), `@"Facet name":value` (SparkFun's curated facets, e.g. "Form Factor", "Sensor Type") |
| Attributes | none in the catalogue (`getattributes` returns `Key: value` bullets when a description has them) | none per product (the GraphQL field errors); use `facets()` |
| Datasheet | PDFs linked under the product page's "Downloads"; otherwise the error points to the Learn guide | datasheet / schematic PDFs linked on the product page (`Part.extra["documents"]`) |
| Pictures | 1200x900, falling back to 640x480 | full gallery |
| 3D models | no (`NoModelError`) | no (`NoModelError`) |
| Not available | `mfr:` (mostly empty), `has:datasheet`, `has:3dmodel`, `lib:`, `is:preferred` | `mfr:`, `is:rohs`, `has:datasheet`, `has:3dmodel`, `lib:`, `is:preferred` |

* **What they are, and are not.** Neither is a parametric catalogue like DigiKey or LCSC: no manufacturer part numbers, no
  attribute tables, no stock counts at SparkFun. They are useful for "which Adafruit / SparkFun board do I buy, what does it
  cost at 10 pieces, is it in stock".
* **Politeness and robots.txt.** Adafruit's `robots.txt` asks crawlers to avoid `/api/` and `/search`; SparkFun's excludes
  its HTML search pages. This is a tool you run, not a crawler: Adafruit's catalogue is fetched once a day, nothing hits a
  search page, and both modules identify themselves as `partsapi` with a 1 s gap between requests. If either shop objects
  to that use, stop using the module.
* **Facets.** `sparkfun.facets("sensor")` returns SparkFun's own filter options with counts. An `@"Name":value` filter that
  matches no single facet raises `ValueError` listing the ones that exist, instead of silently returning nothing.
* **Adafruit categories.** Result lists get a category name through one tiny request per distinct category (remembered).
  `cat:` loads Adafruit's category tree once (about 27 KB cached, fetched from a larger endpoint) and matches the full path.

## TME

Official TME API **v2**, free token + secret from https://developers.tme.eu/ (applications created after 14 May 2026 can
only use v2; older applications use a legacy signed API that this client does not support).

| | |
|---|---|
| Environment | `TME_TOKEN` (the long value), `TME_SECRET` (the short one); optional `TME_COUNTRY=DE`, `TME_CURRENCY=EUR` |
| Auth | OAuth2 client credentials, 5-minute access tokens refreshed automatically |
| Prices | per your TME country: for Germany **gross incl. 19 % VAT** (`PriceBreak.incl_vat`, `vat_rate`) |
| Stock | real unit counts |
| Server-side | search words (2-40 chars), `cat:` (category id, includes sub-categories; unknown names get "similar" suggestions), `mfr:` when it names exactly one manufacturer, `is:instock`, sorting by price (single unit) / code / mpn, **parametric filters** (`res:10k..100k`, `tol:<=1%`, `pkg:0603`, `@"Name":value`) |
| Client-side | everything else, over up to `max_pages` x 100 hits; every hit is completed with prices, stock and parameters (batches of 50) |
| Not available | `is:rohs`, `lib:`, `is:preferred`, `has:3dmodel`; 3D models come from the LCSC/EasyEDA library by exact MPN like the other distributors |

* **Package sizes.** TME lists passives as "Case - inch" `0603` and "Case - mm" `1608`, and metric `0603` is imperial 0201.
  `pkg:0603` therefore means the inch size and never matches the mm facet; ICs use the "Case" parameter (`pkg:so8`).
* **Ambiguous parameter names.** `res` matches "Resistance", "Coil resistance", ...; the parameter whose name *is* the word wins,
  otherwise the condition is applied client-side. Parameter values that are ranges (`3...30V DC`) never satisfy a numeric condition.
* **Category tree.** About 400 KB, fetched once and cached for 7 days (`PARTSAPI_CACHE_DIR`); used for category paths and `cat:`.
* **Verification status.** Run against the real TME API with a real application, including the parametric search.

## How it works, and what to know

* **Unofficial (JLCPCB, LCSC, Reichelt).** These sites publish no API. JLCPCB and LCSC are called at the JSON endpoints
  their own web pages use; Reichelt is parsed from HTML (schema.org microdata). DigiKey and Mouser are official APIs. Any of them can change without notice -
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
or 429 after retries; `.status` has the HTTP code), `ParseError` (site layout changed), `AuthError` (missing or rejected
DigiKey / Mouser credentials), all subclasses of `PartsApiError`.

## Tests

```bash
pytest                               # 347 offline tests against recorded and schema-derived fixtures
PARTSAPI_LIVE=1 pytest -m live       # hits the real sites (DigiKey / Mouser / TME tests need your keys; Adafruit and SparkFun need none)
```
