"""Shared HTTP layer: browser-like headers, throttling, retries and a small TTL cache."""
from __future__ import annotations

import json
import threading
import time
from typing import Any, Optional
from urllib.parse import urlparse

import requests

from .errors import BlockedError, NotFoundError, PartsApiError

USER_AGENT = (
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 "
    "(KHTML, like Gecko) Chrome/148.0.0.0 Safari/537.36"
)
RETRY_STATUS = {429, 500, 502, 503, 504}


class Http:
    """Thin wrapper over ``requests.Session``.

    * ``min_interval`` seconds between requests to the same host (politeness).
    * Up to ``retries`` retries with exponential backoff on 429/5xx and network errors.
    * ``cache_ttl`` seconds of in-memory caching for JSON/text GET and POST-JSON calls.
      Signed asset URLs (JLCPCB) live 30 minutes, so keep the TTL well below that.
    """

    def __init__(
        self,
        session: Optional[requests.Session] = None,
        *,
        timeout: float = 30.0,
        retries: int = 3,
        backoff: float = 1.0,
        min_interval: float = 0.25,
        cache_ttl: float = 300.0,
        headers: Optional[dict[str, str]] = None,
    ) -> None:
        self.session = session or requests.Session()
        self.session.headers.update({"User-Agent": USER_AGENT, "Accept-Language": "en-US,en;q=0.9"})
        if headers:
            self.session.headers.update(headers)
        self.timeout = timeout
        self.retries = retries
        self.backoff = backoff
        self.min_interval = min_interval
        self.cache_ttl = cache_ttl
        self._last: dict[str, float] = {}
        self._cache: dict[str, tuple[float, Any]] = {}
        self._lock = threading.Lock()

    def _throttle(self, url: str) -> None:
        host = urlparse(url).netloc
        with self._lock:
            wait = self._last.get(host, 0.0) + self.min_interval - time.monotonic()
            if wait > 0:
                time.sleep(wait)
            self._last[host] = time.monotonic()

    def request(self, method: str, url: str, **kw: Any) -> requests.Response:
        kw.setdefault("timeout", self.timeout)
        last_exc: Optional[Exception] = None
        for attempt in range(self.retries + 1):
            self._throttle(url)
            try:
                resp = self.session.request(method, url, **kw)
            except requests.RequestException as exc:  # network level
                last_exc = exc
            else:
                if resp.status_code not in RETRY_STATUS:
                    return self._check(resp, url)
                last_exc = BlockedError(f"{url} -> HTTP {resp.status_code}")
            if attempt < self.retries:
                time.sleep(self.backoff * (2**attempt))
        if isinstance(last_exc, BlockedError):
            raise last_exc
        raise PartsApiError(f"request to {url} failed: {last_exc}") from last_exc

    @staticmethod
    def _check(resp: requests.Response, url: str) -> requests.Response:
        if resp.status_code == 404:
            raise NotFoundError(f"{url} -> HTTP 404")
        if resp.status_code in (401, 403, 405) or "Access Denied" in resp.text[:300]:
            raise BlockedError(f"{url} -> HTTP {resp.status_code} (bot protection?)")
        if resp.status_code >= 400:
            raise PartsApiError(f"{url} -> HTTP {resp.status_code}")
        return resp

    # -- cached helpers -------------------------------------------------------------
    def _cached(self, key: str) -> Any:
        hit = self._cache.get(key)
        if hit and time.monotonic() - hit[0] < self.cache_ttl:
            return hit[1]
        return None

    def _store(self, key: str, value: Any) -> Any:
        if self.cache_ttl > 0:
            self._cache[key] = (time.monotonic(), value)
        return value

    def get_json(self, url: str, **kw: Any) -> Any:
        key = f"GET {url} {json.dumps(kw.get('params'), sort_keys=True, default=str)} {kw.get('cookies')}"
        hit = self._cached(key)
        if hit is not None:
            return hit
        return self._store(key, self._json(self.request("GET", url, **kw), url))

    def post_json(self, url: str, body: dict[str, Any], **kw: Any) -> Any:
        key = f"POST {url} {json.dumps(body, sort_keys=True, default=str)}"
        hit = self._cached(key)
        if hit is not None:
            return hit
        return self._store(key, self._json(self.request("POST", url, json=body, **kw), url))

    def get_text(self, url: str, **kw: Any) -> str:
        key = f"TEXT {url} {json.dumps(kw.get('params'), sort_keys=True, default=str)}"
        hit = self._cached(key)
        if hit is not None:
            return hit
        resp = self.request("GET", url, **kw)
        resp.encoding = resp.encoding or "utf-8"
        return self._store(key, resp.text)

    def get_bytes(self, url: str, **kw: Any) -> bytes:
        """Binary download; never cached."""
        return self.request("GET", url, **kw).content

    @staticmethod
    def _json(resp: requests.Response, url: str) -> Any:
        try:
            return resp.json()
        except ValueError as exc:
            raise PartsApiError(f"{url} did not return JSON: {resp.text[:200]!r}") from exc
