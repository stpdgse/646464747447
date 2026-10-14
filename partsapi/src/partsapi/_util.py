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
