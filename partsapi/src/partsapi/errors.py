"""Exceptions raised by partsapi."""
from __future__ import annotations


class PartsApiError(Exception):
    """Base class for every error raised by this package."""


class NotFoundError(PartsApiError):
    """The part (or the requested asset of a part) does not exist."""


class BlockedError(PartsApiError):
    """The site refused the request (bot protection, 403, 429 after retries).

    ``status`` is the HTTP status code when there was one.
    """

    def __init__(self, message: str = "", status: "int | None" = None) -> None:
        super().__init__(message)
        self.status = status


class HttpError(PartsApiError):
    """An HTTP error status other than 404 / 401 / 403 / 405 (those have their own exceptions). ``status`` is the code."""

    def __init__(self, message: str = "", status: "int | None" = None) -> None:
        super().__init__(message)
        self.status = status


class AuthError(PartsApiError):
    """Missing or rejected API credentials (DigiKey client id/secret, Mouser API key)."""


class NoModelError(NotFoundError):
    """No 3D model is available for this part."""


class ParseError(PartsApiError):
    """The site answered, but its response no longer has the expected shape."""
