"""Vercel entry point: serves the Flask app (UI + JSON API) as a single serverless function."""
import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "src"))
os.environ.setdefault("PARTSAPI_CACHE_DIR", "/tmp/partsapi")   # the only writable path on Vercel
os.environ.setdefault("PARTSAPI_NO_DOTENV", "1")               # credentials come from Vercel environment variables

from partsapi.web.app import app as _flask  # noqa: E402
from partsapi.web.wsgi import RestoreOriginalPath  # noqa: E402

app = RestoreOriginalPath(_flask.wsgi_app)  # Vercel looks for `app`
