"""Load API credentials from a ``.env``-style file into ``os.environ`` (never overriding variables that are already set).

Search order (first file that exists wins): the ``path`` argument, ``$PARTSAPI_ENV_FILE``, ``./.env``,
``~/.config/partsapi/env``. Only variables for this package are accepted (``DIGIKEY_*``, ``DK_*``, ``MOUSER_*``, ``TME_*``,
``PARTSAPI_*``), so a stray ``PATH=...`` or ``LD_PRELOAD=...`` line in the file can never change your environment.
Set ``PARTSAPI_NO_DOTENV=1`` to turn the automatic load at ``import partsapi`` off.
"""
from __future__ import annotations

import os
import re
import stat
import warnings
from pathlib import Path
from typing import Optional

PREFIXES = ("DIGIKEY_", "DK_", "MOUSER_", "TME_", "PARTSAPI_")
_LINE = re.compile(r"^\s*(?:export\s+)?([A-Za-z_][A-Za-z0-9_]*)\s*=\s*(.*?)\s*$")


def candidates(path: Optional[os.PathLike] = None) -> list[Path]:
    out: list[Path] = []
    if path:
        out.append(Path(path))
    if os.environ.get("PARTSAPI_ENV_FILE"):
        out.append(Path(os.environ["PARTSAPI_ENV_FILE"]))
    out += [Path.cwd() / ".env", Path.home() / ".config" / "partsapi" / "env"]
    return out


def parse(text: str) -> dict[str, str]:
    values: dict[str, str] = {}
    for raw in text.splitlines():
        m = _LINE.match(raw)
        if not m or raw.lstrip().startswith("#"):
            continue
        key, val = m.groups()
        if len(val) >= 2 and val[0] == val[-1] and val[0] in "'\"":
            val = val[1:-1]
        elif " #" in val:  # trailing comment on an unquoted value
            val = val.split(" #", 1)[0].rstrip()
        values[key] = val
    return values


def load_env(path: Optional[os.PathLike] = None, override: bool = False) -> list[str]:
    """Set credentials from the first env file found; returns the names that were set."""
    for file in candidates(path):
        try:
            if not file.is_file():
                continue
            mode = file.stat().st_mode
            text = file.read_text(encoding="utf-8")
        except OSError:
            continue
        if mode & (stat.S_IRWXG | stat.S_IRWXO):
            warnings.warn(f"{file} is readable by other users; run: chmod 600 {file}", stacklevel=2)
        applied = []
        for key, val in parse(text).items():
            if key.startswith(PREFIXES) and val and (override or key not in os.environ):
                os.environ[key] = val
                applied.append(key)
        return applied
    return []
