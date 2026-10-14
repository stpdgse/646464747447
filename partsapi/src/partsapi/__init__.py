"""partsapi - one API for JLCPCB parts, LCSC, Reichelt, DigiKey, Mouser, Adafruit and SparkFun.

    from partsapi import jlc, lcsc, reichelt, digikey, mouser, adafruit, sparkfun

DigiKey and Mouser use their official APIs and need credentials (see their module docs).
"""
from . import adafruit, digikey, easyeda, jlc, lcsc, mouser, parts, reichelt, sparkfun  # noqa: F401
from .errors import AuthError, BlockedError, NoModelError, NotFoundError, ParseError, PartsApiError
from .models import Model3D, Part, PriceBreak, Stock

__all__ = ["jlc", "lcsc", "reichelt", "digikey", "mouser", "adafruit", "sparkfun", "parts", "easyeda", "Part", "PriceBreak", "Stock", "Model3D",
           "PartsApiError", "NotFoundError", "BlockedError", "NoModelError", "ParseError", "AuthError"]
