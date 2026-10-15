from __future__ import annotations

import re
from pathlib import Path
from typing import Any, Optional, Union
from urllib.parse import urlparse

PathLike = Union[str, Path]
C_CODE = re.compile(r"^C\d{1,9}$", re.I)


def is_lcsc_code(s: str) -> bool:
    return bool(C_CODE.match(s.strip()))


def to_int(v: Any) -> Optional[int]:
    try:
        return int(v)
    except (TypeError, ValueError):
        return None


def to_float(v: Any) -> Optional[float]:
    try:
        return float(v)
    except (TypeError, ValueError):
        return None


def ext_from_url(url: str, default: str = "") -> str:
    suffix = Path(urlparse(url).path).suffix
    return suffix if suffix else default


def save_bytes(data: bytes, save_to: Optional[PathLike], default_name: str) -> Path:
    """Write ``data``; if ``save_to`` is a directory (or ends in a separator) use ``default_name`` inside it."""
    target = Path(save_to) if save_to else Path(default_name)
    if target.is_dir() or str(save_to or "").endswith(("/", "\\")):
        target.mkdir(parents=True, exist_ok=True)
        target = target / default_name
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_bytes(data)
    return target


def safe_name(s: str) -> str:
    return re.sub(r"[^A-Za-z0-9._-]+", "_", s).strip("_") or "part"


def parse_price(text: Any) -> Optional[float]:
    """'0,15 €' -> 0.15, '$1,234.50' -> 1234.5, '1.234,50 €' -> 1234.5, 'ab 2,99 €' -> 2.99, '12' -> 12.0; None if no number.

    The last '.' or ',' that is followed by 1-2 digits at the end is the decimal mark; the other is a thousands mark.
    """
    if text is None:
        return None
    s = str(text).replace("\xa0", " ")
    m = re.search(r"\d[\d.,' ]*", s)
    if not m:
        return None
    num = m.group(0).strip().replace(" ", "").replace("'", "")
    dec = re.search(r"[.,](\d{1,2})$", num)
    if dec:
        whole = re.sub(r"[.,]", "", num[: dec.start()])
        num = f"{whole}.{dec.group(1)}"
    elif re.fullmatch(r"0[.,]\d+", num):  # "0,125": a leading zero is never a thousands group
        num = num.replace(",", ".")
    else:
        num = re.sub(r"[.,]", "", num) if re.fullmatch(r"\d{1,3}([.,]\d{3})+", num) else num.replace(",", ".")
    try:
        return float(num)
    except ValueError:
        return None
