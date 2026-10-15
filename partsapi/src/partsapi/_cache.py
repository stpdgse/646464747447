"""Tiny on-disk JSON cache for big, slowly changing downloads (e.g. Adafruit's catalogue)."""
from __future__ import annotations

import json
import os
import tempfile
import time
from pathlib import Path
from typing import Any, Optional


def default_dir() -> Optional[Path]:
    """``$PARTSAPI_CACHE_DIR`` (``off`` / empty string disables), else ``~/.cache/partsapi``."""
    env = os.environ.get("PARTSAPI_CACHE_DIR")
    if env is not None:
        return None if env.strip().lower() in ("", "off", "none", "0") else Path(env)
    return Path.home() / ".cache" / "partsapi"


class DiskCache:
    def __init__(self, directory: Optional[Path]) -> None:
        self.directory = directory

    def get(self, name: str, max_age: float) -> Any:
        """The cached object, or ``None`` if missing, stale or unreadable."""
        if self.directory is None:
            return None
        path = self.directory / name
        try:
            if time.time() - path.stat().st_mtime > max_age:
                return None
            return json.loads(path.read_text(encoding="utf-8"))
        except (OSError, ValueError):
            return None

    def put(self, name: str, obj: Any) -> None:
        if self.directory is None:
            return
        try:
            self.directory.mkdir(parents=True, exist_ok=True)
            fd, tmp = tempfile.mkstemp(dir=self.directory, prefix=name + ".")
            with os.fdopen(fd, "w", encoding="utf-8") as fh:
                json.dump(obj, fh, separators=(",", ":"), ensure_ascii=False)
            os.replace(tmp, self.directory / name)
        except OSError:
            pass  # a cache that cannot be written is just a cache miss next time
