"""partsapi - one API for JLCPCB parts, LCSC, Reichelt, DigiKey, Mouser, TME, Adafruit and SparkFun.

    from partsapi import jlc, lcsc, reichelt, digikey, mouser, tme, adafruit, sparkfun

DigiKey, Mouser and TME use their official APIs and need credentials (see their module docs).
"""
import os as _os

from ._env import load_env

if not _os.environ.get("PARTSAPI_NO_DOTENV"):
    load_env()  # credentials from ./.env or ~/.config/partsapi/env, before the vendor modules read the environment

from . import adafruit, digikey, easyeda, jlc, lcsc, mouser, parts, reichelt, sparkfun, tme  # noqa: F401
from .errors import AuthError, BlockedError, HttpError, NoModelError, NotFoundError, ParseError, PartsApiError
from .models import Model3D, Part, PriceBreak, Stock

__all__ = ["load_env", "jlc", "lcsc", "reichelt", "digikey", "mouser", "tme", "adafruit", "sparkfun", "parts", "easyeda", "Part", "PriceBreak", "Stock", "Model3D",
           "PartsApiError", "NotFoundError", "BlockedError", "NoModelError", "ParseError", "AuthError", "HttpError"]
