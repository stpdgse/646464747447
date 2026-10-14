import os
import subprocess
import sys

import pytest

from partsapi import _env
from partsapi._env import load_env, parse


@pytest.fixture
def clean(monkeypatch, tmp_path):
    for k in list(os.environ):
        if k.startswith(_env.PREFIXES):
            monkeypatch.delenv(k)
    monkeypatch.delenv("PARTSAPI_ENV_FILE", raising=False)
    monkeypatch.setenv("HOME", str(tmp_path / "home"))
    monkeypatch.chdir(tmp_path)
    return tmp_path


def write(path, text, mode=0o600):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text)
    path.chmod(mode)
    return path


def test_parse_formats():
    text = """
# a comment
TME_TOKEN=abc123
export TME_SECRET = "quoted value"
MOUSER_API_KEY='single'
DK_CLIENT_ID=with # trailing comment
  DK_CLIENT_SECRET=spaced   
EMPTY=
not a line
=novalue
"""
    assert parse(text) == {"TME_TOKEN": "abc123", "TME_SECRET": "quoted value", "MOUSER_API_KEY": "single", "DK_CLIENT_ID": "with",
                           "DK_CLIENT_SECRET": "spaced", "EMPTY": ""}


def test_load_sets_only_package_variables(clean):
    f = write(clean / ".env", "TME_TOKEN=t\nMOUSER_API_KEY=m\nPATH=/evil\nLD_PRELOAD=/evil.so\nHOME=/evil\nDK_CLIENT_ID=d\nPARTSAPI_CACHE_DIR=/c\n")
    old_path = os.environ["PATH"]
    assert sorted(load_env()) == ["DK_CLIENT_ID", "MOUSER_API_KEY", "PARTSAPI_CACHE_DIR", "TME_TOKEN"]
    assert os.environ["TME_TOKEN"] == "t" and os.environ["PATH"] == old_path and "LD_PRELOAD" not in os.environ
    assert os.environ["HOME"] == str(clean / "home")
    assert f.exists()


def test_existing_environment_wins_unless_override(clean, monkeypatch):
    write(clean / ".env", "TME_TOKEN=from-file\nTME_SECRET=s\n")
    monkeypatch.setenv("TME_TOKEN", "from-env")
    assert load_env() == ["TME_SECRET"] and os.environ["TME_TOKEN"] == "from-env"
    assert load_env(override=True) == ["TME_TOKEN", "TME_SECRET"] and os.environ["TME_TOKEN"] == "from-file"


def test_search_order_and_first_file_wins(clean, monkeypatch):
    write(clean / "home" / ".config" / "partsapi" / "env", "TME_TOKEN=home\n")
    assert load_env() == ["TME_TOKEN"] and os.environ["TME_TOKEN"] == "home"
    monkeypatch.delenv("TME_TOKEN")
    write(clean / ".env", "TME_TOKEN=cwd\n")
    load_env()
    assert os.environ["TME_TOKEN"] == "cwd"
    monkeypatch.delenv("TME_TOKEN")
    write(clean / "custom.env", "TME_TOKEN=var\n")
    monkeypatch.setenv("PARTSAPI_ENV_FILE", str(clean / "custom.env"))
    load_env()
    assert os.environ["TME_TOKEN"] == "var"
    monkeypatch.delenv("TME_TOKEN")
    write(clean / "explicit.env", "TME_TOKEN=arg\n")
    load_env(clean / "explicit.env")
    assert os.environ["TME_TOKEN"] == "arg"


def test_missing_or_unreadable_files_are_not_errors(clean):
    assert load_env() == [] and load_env(clean / "nope.env") == []
    (clean / ".env").mkdir()                                               # a directory called .env
    assert load_env() == []


def test_warns_when_the_file_is_readable_by_others(clean):
    write(clean / ".env", "TME_TOKEN=t\n", mode=0o644)
    with pytest.warns(UserWarning, match="chmod 600"):
        load_env()
    os.environ.pop("TME_TOKEN")
    (clean / ".env").chmod(0o600)
    import warnings
    with warnings.catch_warnings():
        warnings.simplefilter("error")
        load_env()


def test_import_loads_the_file_and_can_be_switched_off(clean):
    env_file = write(clean / "creds.env", "TME_TOKEN=tok\nTME_SECRET=sec\n")
    base = {"PATH": os.environ["PATH"], "HOME": str(clean / "home"), "PARTSAPI_ENV_FILE": str(env_file)}
    code = "import partsapi; from partsapi import tme; print(tme.is_configured())"
    run = lambda extra: subprocess.run([sys.executable, "-c", code], env={**base, **extra}, capture_output=True, text=True, cwd=clean).stdout.strip()  # noqa: E731
    assert run({}) == "True"
    assert run({"PARTSAPI_NO_DOTENV": "1"}) == "False"


def test_env_example_lists_every_variable_and_no_values():
    from pathlib import Path
    ex = (Path(__file__).parent.parent / ".env.example").read_text()
    for name in ("DIGIKEY_CLIENT_ID", "DIGIKEY_CLIENT_SECRET", "MOUSER_API_KEY", "TME_TOKEN", "TME_SECRET"):
        assert f"\n{name}=\n" in ex
