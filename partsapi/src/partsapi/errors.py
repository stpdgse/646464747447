"""Exceptions raised by partsapi."""
from __future__ import annotations


class PartsApiError(Exception):
    """Base class for every error raised by this package."""


class NotFoundError(PartsApiError):
    """The part (or the requested asset of a part) does not exist."""


class BlockedError(PartsApiError):
    """The site refused the request (bot protection, 403, 429 after retries)."""


class NoModelError(NotFoundError):
    """No 3D model is available for this part."""


class ParseError(PartsApiError):
    """The site answered, but its response no longer has the expected shape."""
