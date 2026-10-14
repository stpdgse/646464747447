import pytest

from conftest import FakeResponse, FakeSession, make_http
from partsapi.errors import BlockedError, NotFoundError, PartsApiError
from partsapi.models import PriceBreak, pick_price


def test_pick_price_uses_highest_reached_rung():
    b = [PriceBreak(1, 99, 1.0), PriceBreak(100, 999, 0.5), PriceBreak(1000, None, 0.25)]
    assert pick_price(b, 1) == 1.0
    assert pick_price(b, 99) == 1.0
    assert pick_price(b, 100) == 0.5
    assert pick_price(b, 5000) == 0.25


def test_pick_price_below_first_rung_uses_first_and_validates_qty():
    b = [PriceBreak(10, 99, 2.0), PriceBreak(100, None, 1.0)]
    assert pick_price(b, 1) == 2.0
    assert pick_price([], 5) is None
    with pytest.raises(ValueError):
        pick_price(b, 0)


def test_retry_then_success():
    s = FakeSession()
    answers = iter([FakeResponse(503), FakeResponse(200, json_data={"ok": 1})])
    s.add("GET", "x.test", lambda **kw: next(answers))
    assert make_http(s).get_json("https://x.test/a") == {"ok": 1}
    assert len(s.calls) == 2


def test_retries_exhausted_raises_blocked():
    s = FakeSession().add("GET", "x.test", FakeResponse(429))
    with pytest.raises(BlockedError):
        make_http(s).get_json("https://x.test/a")
    assert len(s.calls) == 3  # 1 try + 2 retries


@pytest.mark.parametrize("status,exc", [(404, NotFoundError), (403, BlockedError), (405, BlockedError), (418, PartsApiError)])
def test_status_mapping(status, exc):
    s = FakeSession().add("GET", "x.test", FakeResponse(status, text="nope"))
    with pytest.raises(exc):
        make_http(s).get_json("https://x.test/a")


def test_access_denied_body_is_blocked_even_on_200():
    s = FakeSession().add("GET", "x.test", FakeResponse(200, text="<HTML><TITLE>Access Denied</TITLE>"))
    with pytest.raises(BlockedError):
        make_http(s).get_json("https://x.test/a")


def test_json_cache_and_non_json_error():
    s = FakeSession().add("GET", "x.test", FakeResponse(200, json_data={"a": 1}))
    h = make_http(s)
    h.get_json("https://x.test/a")
    h.get_json("https://x.test/a")
    assert len(s.calls) == 1
    s2 = FakeSession().add("GET", "x.test", FakeResponse(200, text="<html>"))
    with pytest.raises(PartsApiError):
        make_http(s2).get_json("https://x.test/a")


def test_bytes_not_cached():
    s = FakeSession().add("GET", "x.test", FakeResponse(200, content=b"abc"))
    h = make_http(s)
    assert h.get_bytes("https://x.test/f") == b"abc"
    h.get_bytes("https://x.test/f")
    assert len(s.calls) == 2
