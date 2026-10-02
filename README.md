# Automated PCB toolchain

One script sets up a headless, scriptable PCB workflow on Ubuntu 24.04 (x86_64):

| Tool | Purpose |
|---|---|
| KiCad 9.0 (PCBNew, `kicad-cli`) | Board editor, DRC, Gerber/drill export, Python `pcbnew` API |
| SKiDl | Describe circuits in Python, emit KiCad netlists |
| Fabrication Toolkit (Benny Megidish) | JLCPCB-ready Gerber / BOM / CPL output |
| Freerouting | Headless autorouter (Specctra DSN in, SES out) |

## Install

```bash
sudo ./setup-eda.sh            # install everything, then run the smoke test
sudo ./setup-eda.sh --no-test  # install only
sudo ./setup-eda.sh --test     # smoke test only
```

The script is idempotent. Open a new shell afterwards (or `source /etc/profile.d/eda.sh`).

## Pipeline

```
SKiDl script ──► netlist ──► PCBNew (place, outline) ──► DSN ──► Freerouting ──► SES
                                                                                  │
                        JLCPCB files ◄── Fabrication Toolkit ◄── routed .kicad_pcb ◄┘
```

Route a board headless:

```bash
freerouting -de board.dsn -do board.ses --gui.enabled=false
```

## Gotchas this script already handles

- SKiDl defaults to KiCad 10 mode; scripts must call `set_default_tool(KICAD9)`.
- SKiDl needs `KICAD9_SYMBOL_DIR` and KiCad's global `fp-lib-table`, which KiCad only
  creates on first GUI launch. The script sets both up.
- SKiDl's dependencies fail to build with the distro's patched pip, so it lives in a venv
  (`/opt/eda/venv`) on the system Python 3.12, which is the interpreter `pcbnew` is built for.
- Freerouting 2.4.1 needs Java 25; the default Java 21 fails with `UnsupportedClassVersionError`.
- The Fabrication Toolkit download is verified against the SHA-256 published in KiCad's
  plugin index before it is installed.
