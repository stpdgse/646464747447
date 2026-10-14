from __future__ import annotations

import json
from pathlib import Path
from typing import Any, Callable, Optional

import pytest

from partsapi._http import Http

FIX = Path(__file__).parent / "fixtures"


def fixture_text(name: str) -> str:
    return (FIX / name).read_text(encoding="utf-8")


def fixture_json(name: str) -> Any:
    return json.loads(fixture_text(name))


class FakeResponse:
    def __init__(self, status: int = 200, *, text: str = "", content: Optional[bytes] = None, json_data: Any = None):
        self.status_code = status
        self._json = json_data
        self.content = content if content is not None else (json.dumps(json_data).encode() if json_data is not None else text.encode())
        self.text = text if text else (json.dumps(json_data) if json_data is not None else self.content.decode("utf-8", "replace"))
        self.encoding = "utf-8"
        self.headers: dict[str, str] = {}

    def json(self) -> Any:
        if self._json is None:
            raise ValueError("not json")
        return self._json


class FakeSession:
    """Route table: first route whose method matches and whose substring is in the URL wins."""

    def __init__(self) -> None:
        self.headers: dict[str, str] = {}
        self.routes: list[tuple[str, str, Callable[..., FakeResponse]]] = []
        self.calls: list[dict[str, Any]] = []

    def add(self, method: str, url_part: str, responder: "Callable[..., FakeResponse] | FakeResponse") -> "FakeSession":
        fn = responder if callable(responder) else (lambda **kw: responder)
        self.routes.append((method.upper(), url_part, fn))
        return self

    def request(self, method: str, url: str, **kw: Any) -> FakeResponse:
        self.calls.append({"method": method.upper(), "url": url, **kw})
        for m, part, fn in self.routes:
            if m == method.upper() and part in url:
                return fn(url=url, **kw)
        raise AssertionError(f"unrouted request: {method} {url}")


def make_http(session: FakeSession) -> Http:
    return Http(session, retries=2, backoff=0, min_interval=0, cache_ttl=60)


@pytest.fixture
def session() -> FakeSession:
    return FakeSession()


@pytest.fixture
def http(session: FakeSession) -> Http:
    return make_http(session)


def pytest_collection_modifyitems(config, items):
    import os

    if os.environ.get("PARTSAPI_LIVE") == "1":
        return
    skip = pytest.mark.skip(reason="live test; set PARTSAPI_LIVE=1")
    for item in items:
        if "live" in item.keywords:
            item.add_marker(skip)
