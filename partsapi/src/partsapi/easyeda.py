"""EasyEDA / JLCEDA library access: LCSC-catalogue keyword search and 3D models.

Every LCSC "C" number (and therefore every JLCPCB part) has an EasyEDA component. Its PCB footprint
carries a 3D model node whose UUID resolves to downloadable OBJ and STEP files.

Verified live (2026-10): ``/3dmodel/<uuid>`` serves the **OBJ**, ``/<key>/<uuid>`` serves the **STEP**.

The EasyEDA/JLCEDA library is copyrighted by JLCEDA; anything redistributing its content must credit
"JLCEDA/EasyEDA Official Library" with links to https://lceda.cn/ and https://easyeda.com. :class:`Model3D`
carries that credit in ``attribution``.
"""
from __future__ import annotations

import json
from typing import Any, Optional

from ._http import Http
from ._util import PathLike, is_lcsc_code, safe_name, save_bytes
from .errors import NoModelError, NotFoundError, ParseError
from .models import Model3D

BASE = "https://easyeda.com"
MODULES = "https://modules.easyeda.com"
STEP_KEY = "qAxj6KHrDKw4blvCG8QJPs7Y"
LCSC_OWNER_UUID = "0819f05c4eef4c71ace90d822a990e87"
EDITOR_VERSION = "6.5.51"


class EasyEDA:
    def __init__(self, http: Optional[Http] = None) -> None:
        self.http = http or Http()

    # -- components -------------------------------------------------------------------
    def component(self, lcsc_code: str) -> dict[str, Any]:
        """Raw EasyEDA component (symbol + footprint data) for an LCSC C-number."""
        code = lcsc_code.strip().upper()
        if not is_lcsc_code(code):
            raise ValueError(f"{lcsc_code!r} is not an LCSC part number (expected e.g. 'C25804')")
        data = self.http.get_json(
            f"{BASE}/api/products/{code}/components", params={"version": EDITOR_VERSION}
        )
        if not data.get("success") or not data.get("result"):
            raise NotFoundError(f"EasyEDA has no component for {code}")
        return data["result"]

    def search(self, keyword: str, limit: int = 25, page: int = 1) -> list[dict[str, Any]]:
        """Keyword search of the LCSC library as exposed by EasyEDA.

        Returns light dicts: ``code, mpn, manufacturer, package, library_class, title, smt, uuid``.
        ``page`` is 1-based; one page holds at most 100 hits.
        """
        body = {
            "type": "3",
            "doctype[]": "2",
            "uid": LCSC_OWNER_UUID,
            "returnListStyle": "classifyarr",
            "wd": keyword,
            "version": EDITOR_VERSION,
            "page": str(page),
            "pageSize": str(max(1, min(limit, 100))),
        }
        resp = self.http.request("POST", f"{BASE}/api/components/search", data=body)
        payload = self.http._json(resp, f"{BASE}/api/components/search")
        if not payload.get("success"):
            raise ParseError(f"EasyEDA search failed: {str(payload)[:200]}")
        out: list[dict[str, Any]] = []
        for item in payload["result"].get("lists", {}).get("lcsc", []):
            para = (item.get("dataStr") or {}).get("head", {}).get("c_para", {})
            code = para.get("Supplier Part")
            if not code:
                continue
            out.append(
                {
                    "code": code,
                    "mpn": para.get("Manufacturer Part"),
                    "manufacturer": para.get("Manufacturer"),
                    "package": para.get("package"),
                    "library_class": para.get("JLCPCB Part Class"),
                    "title": item.get("title"),
                    "smt": bool(item.get("SMT")),
                    "uuid": item.get("uuid"),
                }
            )
            if len(out) >= limit:
                break
        return out

    # -- 3D models --------------------------------------------------------------------
    @staticmethod
    def model_uuid(component: dict[str, Any]) -> Optional[str]:
        """Find the 3D-model UUID inside a component's footprint, or ``None`` if it has none."""
        pkg = component.get("packageDetail") or {}
        data_str = pkg.get("dataStr") or {}
        for shape in data_str.get("shape") or []:
            if not isinstance(shape, str) or not shape.startswith("SVGNODE~"):
                continue
            try:
                attrs = json.loads(shape.split("~", 1)[1]).get("attrs", {})
            except (ValueError, AttributeError):
                continue
            if attrs.get("c_etype") == "outline3D" and attrs.get("uuid"):
                return attrs["uuid"]
        legacy = (data_str.get("head") or {}).get("uuid_3d")
        return legacy or None

    def get3dmodel_by_mpn(self, mpn: str, fmt: str = "step", save_to: Optional[PathLike] = None) -> Model3D:
        """3D model for a part from any vendor, matched on an **exact** manufacturer part number (case, spaces
        and dashes ignored) against the LCSC library. ``Model3D.source_code`` names the LCSC part used."""
        norm = lambda s: "".join(ch for ch in (s or "").upper() if ch.isalnum())  # noqa: E731
        hits = [h for h in self.search(mpn, limit=50) if norm(h.get("mpn")) == norm(mpn)]
        for h in hits:
            try:
                return self.get3dmodel(h["code"], fmt, save_to)
            except NoModelError:
                continue
        raise NoModelError(f"no 3D model found in the EasyEDA/LCSC library for MPN {mpn!r} ({len(hits)} exact match(es))")

    def get3dmodel(self, lcsc_code: str, fmt: str = "step", save_to: Optional[PathLike] = None) -> Model3D:
        fmt = fmt.lower()
        if fmt not in ("step", "stp", "obj"):
            raise ValueError("fmt must be 'step' or 'obj'")
        fmt = "step" if fmt == "stp" else fmt
        code = lcsc_code.strip().upper()
        uuid = self.model_uuid(self.component(code))
        if not uuid:
            raise NoModelError(f"{code} has no 3D model in the EasyEDA library")
        url = f"{MODULES}/3dmodel/{uuid}" if fmt == "obj" else f"{MODULES}/{STEP_KEY}/{uuid}"
        data = self.http.get_bytes(url)
        marker = b"ISO-10303" if fmt == "step" else None
        if not data or (marker and marker not in data[:64]) or (fmt == "obj" and b"v " not in data[:4096]):
            raise NoModelError(f"{code}: model file for uuid {uuid} is empty or not a valid {fmt.upper()}")
        model = Model3D(data=data, format=fmt, filename=f"{safe_name(code)}.{fmt if fmt == 'obj' else 'step'}",
                        source_code=code, uuid=uuid)
        if save_to is not None:
            save_bytes(data, save_to, model.filename)
        return model
