"""``python -m partsapi.web [--host 127.0.0.1] [--port 8000]`` - run the UI locally."""
import argparse
import os

from .app import app


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--host", default="127.0.0.1")
    ap.add_argument("--port", type=int, default=8000)
    args = ap.parse_args()
    if args.host not in ("127.0.0.1", "localhost", "::1") and not os.environ.get("WEBUI_PASSWORD"):
        raise SystemExit("refusing to listen on a public address without WEBUI_PASSWORD set")
    app.run(host=args.host, port=args.port, threaded=True)


if __name__ == "__main__":
    main()
