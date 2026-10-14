"""Reichelt Elektronik (https://www.reichelt.de) - search, price, stock, pictures, datasheets, 3D.

Reichelt has no public API, so this parses the shop's HTML (schema.org microdata + page markup).
Prices are **EUR gross, including German VAT** (19 % unless the page says otherwise) - that is what the
shop displays to private customers. Stock is reported as in/out of stock plus the delivery estimate;
Reichelt does not publish unit counts. Search returns the first result page only.

    from partsapi import reichelt
    reichelt.findpart("LM358", limit=5)
    reichelt.getprice("LM358DT")                 # unit price, EUR incl. VAT
    reichelt.getstock("LM358DT")
    reichelt.getpic("LM358DT", save_to="out/")
    reichelt.getdatasheet("LM358DT", save_to="out/")
    reichelt.get3dmodel("LM358DT")               # resolved via the MPN in the EasyEDA/LCSC library
"""
from __future__ import annotations

import re
from pathlib import Path
from typing import Any, Callable, Optional, Union
from urllib.parse import quote, urljoin

from bs4 import BeautifulSoup
from bs4.element import Tag

from . import easyeda as _easyeda
from ._http import Http
from ._util import PathLike, ext_from_url, safe_name, save_bytes
from .errors import NoModelError, NotFoundError, ParseError
from .filters import ParsedQuery, PartFilter, build_filter, facets as _facets, parse_query, sort_parts
from .models import Model3D, Part, PriceBreak, Stock, pick_price

BASE = "https://www.reichelt.de"
DEFAULT_LOCALE = "de/de"
# check groups a search result card can answer without opening the product page
_CARD_HAS = ("text", "code", "manufacturer", "package", "description", "price", "attrs", "image")


def parse_price(text: str) -> Optional[float]:
    """'0,15 €' -> 0.15, '1.234,50 €' -> 1234.5, 'ab 2,99 €' -> 2.99. None if no number."""
    m = re.search(r"\d[\d.]*(?:,\d+)?|\d+", text.replace("\xa0", " "))
    if not m:
        return None
    num = m.group(0)
    num = num.replace(".", "").replace(",", ".") if "," in num else num
    try:
        return float(num)
    except ValueError:
        return None


def _norm(s: str) -> str:
    return re.sub(r"[\s_\-]+", "", s or "").upper()


def _meta(node: Tag, prop: str) -> Optional[str]:
    el = node.find(attrs={"itemprop": prop})
    if el is None:
        return None
    val = el.get("content") if el.name == "meta" else el.get_text(" ", strip=True)
    return (val or "").strip() or None


class Reichelt:
    def __init__(self, http: Optional[Http] = None, locale: str = DEFAULT_LOCALE) -> None:
        self.http = http or Http(headers={"Accept-Language": "de-DE,de;q=0.9,en;q=0.5"}, min_interval=0.5)
        self.locale = locale.strip("/")
        self._urls: dict[str, str] = {}
        self._easyeda = _easyeda.EasyEDA(self.http)

    # -- search -----------------------------------------------------------------------
    def _search_url(self, query: str) -> str:
        return f"{BASE}/{self.locale}/shop/suche/{quote(query.strip(), safe='')}"

    def findpart(
        self,
        query: str,
        *,
        limit: int = 30,
        where: Union[None, str, PartFilter] = None,
        sort: Union[None, str, Callable[[Part], Any]] = None,
        reverse: bool = False,
        qty: int = 1,
        max_fetch: int = 30,
        parse: bool = True,
        strict: Optional[bool] = None,
    ) -> list[Part]:
        """Search the shop, with the full filter syntax of :mod:`partsapi.filters`.

        Reichelt returns one result page (about 10-30 articles). Manufacturer, package, price, description
        and the attribute table on the result cards are filtered directly; if the filter also needs
        stock, datasheet, MPN or RoHS, the surviving articles' product pages are fetched (at most
        ``max_fetch``). Attribute names are German ("Bauform", "Versorgungsspannung"); the aliases
        ``res cap ind tol power volt curr freq temp`` know the German names. ``lib:`` / ``is:preferred`` do not exist here.
        """
        if not query.strip() and not where:
            raise ValueError("empty query")
        pq = parse_query(query, qty=qty, strict=strict) if parse else ParsedQuery([query] if query.strip() else [], [], PartFilter(qty=qty))
        flt = pq.filter
        if where is not None:
            flt.merge(build_filter(where, qty))
        if flt.flags & {"preferred", "basic", "extended"} or any(n == "library" for n, _ in flt.fields):
            raise ValueError("lib:/is:preferred are JLCPCB concepts and do not exist on Reichelt")
        queries = [k for k in pq.server_queries() if k.strip()]
        if not queries:
            raise ValueError("Reichelt search needs at least one search word")
        card_skip = {"mpn", "category", "datasheet", "stock", "rohs", "library"}
        needs_detail = flt.requires_detail(set(_CARD_HAS))
        found: dict[str, Part] = {}
        for kw in queries:
            html = self.http.get_text(self._search_url(kw))
            for card in BeautifulSoup(html, "lxml").select("div.al_gallery_article"):
                part = self._parse_card(card)
                if part is None or part.code in found:
                    continue
                self._urls[_norm(part.code)] = part.url or ""
                if flt.matches(part, card_skip):
                    found[part.code] = part
        parts = list(found.values())
        if needs_detail:
            full: list[Part] = []
            for part in parts[:max_fetch]:
                try:
                    detail = self.getpart(part.code)
                except NotFoundError:
                    continue
                if flt.matches(detail):
                    full.append(detail)
            parts = full
        if sort:
            parts = sort_parts(parts, sort, reverse=reverse, qty=qty)
        return parts[:limit]

    def facets(self, query: str, **kw: Any) -> dict[str, Any]:
        """Counts over the search results (computed locally)."""
        return _facets(self.findpart(query, **kw))

    def _parse_card(self, card: Tag) -> Optional[Part]:
        code = _meta(card, "productID")
        link = card.select_one("a.al_artinfo_link") or card.select_one("a[itemprop=url]")
        if not code or link is None or not link.get("href"):
            return None
        url = urljoin(BASE, link["href"])
        img = card.select_one("img[itemprop=image]")
        image = (img.get("data-src") or img.get("src")) if img else None
        price_el = card.select_one(".productPrice")
        price = parse_price(price_el.get_text("", strip=True)) if price_el else None
        attrs: dict[str, str] = {}
        for li in card.select(".al_artinfo_props li"):
            cells = [p.get_text(" ", strip=True) for p in li.find_all("p")]
            if len(cells) == 2:
                attrs[cells[0]] = cells[1]
        # In search cards Reichelt swaps the semantics: itemprop=sku holds the manufacturer name.
        return Part(
            vendor="reichelt",
            code=code,
            manufacturer=_meta(card, "sku"),
            description=_meta(card, "name"),
            package=attrs.get("Bauform") or attrs.get("Gehäuse"),
            url=url,
            image_urls=[image] if image else [],
            price_breaks=[PriceBreak(1, None, price, "EUR", True, 0.19)] if price is not None else [],
            attributes=attrs,
            extra={"search_card": True},
        )

    # -- single part ---------------------------------------------------------------------
    def _resolve_url(self, code: str) -> str:
        if code.startswith(("http://", "https://")):
            return code
        cached = self._urls.get(_norm(code))
        if cached:
            return cached
        results = self.findpart(code, limit=50)
        for p in results:
            if _norm(p.code) == _norm(code):
                return p.url or ""
        hint = ", ".join(p.code for p in results[:5]) or "no results"
        raise NotFoundError(f"Reichelt has no article numbered {code!r} (search gave: {hint})")

    def getpart(self, code: str) -> Part:
        """Full details for a Reichelt article number (e.g. ``'LM358DT'``) or a product-page URL."""
        url = self._resolve_url(code.strip())
        soup = BeautifulSoup(self.http.get_text(url), "lxml")
        return self._parse_product(soup, url)

    def _parse_product(self, soup: BeautifulSoup, url: str) -> Part:
        h1 = soup.find("h1", attrs={"itemprop": "name"}) or soup.find("h1")
        sku_el = soup.find(attrs={"itemprop": "sku"})
        if h1 is None or sku_el is None:
            raise ParseError(f"{url}: not a Reichelt product page (layout changed?)")
        code = sku_el.get_text(strip=True)
        attrs: dict[str, str] = {}
        for ul in soup.select("ul.articleAttribute"):
            cells = [li.get_text(" ", strip=True) for li in ul.find_all("li", recursive=False)]
            if len(cells) == 2 and cells[0]:
                attrs[cells[0]] = cells[1]
        # price
        price_el = soup.find("meta", attrs={"itemprop": "price"})
        price = float(price_el["content"]) if price_el and price_el.get("content") else None
        cur_el = soup.find("meta", attrs={"itemprop": "priceCurrency"})
        currency = cur_el["content"] if cur_el and cur_el.get("content") else "EUR"
        vat = re.search(r"inkl\.\s*(\d+)\s*%\s*(?:gesetzl\.\s*)?MwSt", soup.get_text(" "))
        vat_rate = int(vat.group(1)) / 100 if vat else 0.19
        # availability
        avail = soup.find("link", attrs={"itemprop": "availability"})
        avail_href = (avail.get("href") if avail else "") or ""
        in_stock = avail_href.rstrip("/").endswith("InStock")
        delivery_el = soup.select_one('a[data-event-listener*="open-modal-availability"]')
        delivery = delivery_el.get_text(" ", strip=True) if delivery_el else None
        # images (largest first), datasheets
        images: list[str] = []
        og = soup.find("meta", attrs={"name": "og:image"}) or soup.find("meta", attrs={"property": "og:image"})
        for src in [i.get("src") or i.get("data-src") for i in soup.select("#img-container img, .swiper-slide img")] + [og.get("content") if og else None]:
            if src and src not in images and not src.endswith(".svg"):
                images.append(urljoin(BASE, src))
        images.sort(key=lambda u: 0 if "xxl_ws" in u else 1)  # stable: biggest renders first
        datasheets = [urljoin(BASE, a["href"]) for a in soup.select('a[href$=".pdf"]')]
        cats = [s.get_text(strip=True) for s in soup.select('li[itemprop="itemListElement"] a span[itemprop="name"]')][1:]
        mpn = _meta(soup, "mpn")
        return Part(
            vendor="reichelt",
            code=code,
            mpn=mpn,
            manufacturer=_meta(soup, "brand"),
            description=h1.get_text(" ", strip=True),
            category=" / ".join(cats) or None,
            package=attrs.get("Bauform") or attrs.get("Gehäuse"),
            url=url,
            datasheet_url=datasheets[0] if datasheets else None,
            image_urls=images,
            price_breaks=[PriceBreak(1, None, price, currency, True, vat_rate)] if price is not None else [],
            stock=Stock("reichelt", code, None, in_stock, delivery, {"availability": avail_href.rsplit("/", 1)[-1] or None}),
            attributes=attrs,
            extra={"ean": _meta(soup, "gtin13"), "datasheets": datasheets, "price_note": f"gross, incl. {vat_rate:.0%} VAT"},
            raw={"url": url},
        )

    # -- focused getters -------------------------------------------------------------------
    def getpartprice(self, code: str, qty: Optional[int] = None) -> Union[list[PriceBreak], float]:
        """Unit price (EUR incl. VAT). Reichelt shows one unit price on the page, so ``qty`` does not change it."""
        breaks = self.getpart(code).price_breaks
        if not breaks:
            raise NotFoundError(f"{code} has no price on Reichelt")
        return breaks if qty is None else pick_price(breaks, qty)  # type: ignore[return-value]

    getprice = getpartprice

    def getstock(self, code: str) -> Stock:
        s = self.getpart(code).stock
        assert s is not None
        return s

    def getattributes(self, code: str) -> dict[str, str]:
        return self.getpart(code).attributes

    def getpics(self, code: str) -> list[str]:
        return self.getpart(code).image_urls

    def getpic(self, code: str, save_to: Optional[PathLike] = None, index: int = 0) -> Union[bytes, Path]:
        part = self.getpart(code)
        if not part.image_urls:
            raise NotFoundError(f"{code} has no picture on Reichelt")
        url = part.image_urls[min(index, len(part.image_urls) - 1)]
        data = self.http.get_bytes(url)
        return save_bytes(data, save_to, f"{safe_name(part.code)}{ext_from_url(url, '.jpg')}") if save_to else data

    def getdatasheet(self, code: str, save_to: Optional[PathLike] = None) -> Union[bytes, Path]:
        part = self.getpart(code)
        if not part.datasheet_url:
            raise NotFoundError(f"{code} has no datasheet on Reichelt")
        data = self.http.get_bytes(part.datasheet_url)
        return save_bytes(data, save_to, f"{safe_name(part.code)}.pdf") if save_to else data

    def get3dmodel(self, code: str, fmt: str = "step", save_to: Optional[PathLike] = None) -> Model3D:
        """Reichelt hosts no 3D models. The part's manufacturer part number is matched exactly against the
        LCSC/EasyEDA library and that model is returned (see ``Model3D.source_code``)."""
        part = self.getpart(code)
        mpn = part.mpn
        if not mpn:
            raise NoModelError(f"{code}: Reichelt lists no manufacturer part number to look a model up by")
        wanted = _norm(mpn)
        hits = [h for h in self._easyeda.search(mpn, limit=50) if _norm(h.get("mpn") or "") == wanted]
        for h in hits:
            try:
                return self._easyeda.get3dmodel(h["code"], fmt, save_to)
            except NoModelError:
                continue
        raise NoModelError(f"no 3D model found in the EasyEDA/LCSC library for MPN {mpn!r} ({len(hits)} exact match(es))")


_default = Reichelt()


def findpart(query: str, **kw: Any) -> list[Part]: return _default.findpart(query, **kw)
def facets(query: str, **kw: Any) -> dict[str, Any]: return _default.facets(query, **kw)
def getpart(code: str) -> Part: return _default.getpart(code)
def getpartprice(code: str, qty: Optional[int] = None): return _default.getpartprice(code, qty)
getprice = getpartprice
def getstock(code: str) -> Stock: return _default.getstock(code)
def getattributes(code: str) -> dict[str, str]: return _default.getattributes(code)
def getpics(code: str) -> list[str]: return _default.getpics(code)
def getpic(code: str, save_to: Optional[PathLike] = None, index: int = 0): return _default.getpic(code, save_to, index)
def getdatasheet(code: str, save_to: Optional[PathLike] = None): return _default.getdatasheet(code, save_to)
def get3dmodel(code: str, fmt: str = "step", save_to: Optional[PathLike] = None) -> Model3D: return _default.get3dmodel(code, fmt, save_to)
