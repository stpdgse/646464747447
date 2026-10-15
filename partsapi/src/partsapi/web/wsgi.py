"""WSGI helper for hosts that rewrite every URL to one function (Vercel): restore the original request path.

``vercel.json`` rewrites ``/api/x`` to ``/api/index?__path=api/x``; this wrapper puts ``/api/x`` back into ``PATH_INFO`` and removes the
helper parameter, so Flask routes (and the access-key gate, which looks at the path) see the URL the browser asked for.
"""
from __future__ import annotations

from typing import Any, Callable, Iterable
from urllib.parse import parse_qsl, urlencode

PARAM = "__path"


class RestoreOriginalPath:
    def __init__(self, wsgi_app: Callable[..., Iterable[bytes]]) -> None:
        self.wsgi_app = wsgi_app

    def __call__(self, environ: dict[str, Any], start_response: Callable[..., Any]) -> Iterable[bytes]:
        pairs = parse_qsl(environ.get("QUERY_STRING", ""), keep_blank_values=True)
        original = [v for k, v in pairs if k == PARAM]
        if original:
            environ["PATH_INFO"] = "/" + original[0].lstrip("/")
            environ["SCRIPT_NAME"] = ""
            environ["QUERY_STRING"] = urlencode([(k, v) for k, v in pairs if k != PARAM])
        return self.wsgi_app(environ, start_response)
