#!/usr/bin/env bash
# setup-eda.sh — automated PCB toolchain for Ubuntu 24.04 (x86_64), run as root.
#
# Installs:
#   * KiCad 9.0 (incl. PCBNew, kicad-cli, symbol/footprint/3D libraries)
#   * SKiDl (Python venv at /opt/eda/venv, with access to KiCad's pcbnew module)
#   * Fabrication Toolkit by Benny Megidish (JLCPCB output plugin)
#   * Freerouting (headless autorouter, plus the Java 25 runtime it needs)
#
# Safe to re-run: every step checks whether it is already done.
#
# Usage:  sudo ./setup-eda.sh            install everything, then run smoke test
#         sudo ./setup-eda.sh --no-test  install only
#         sudo ./setup-eda.sh --test     smoke test only
set -euo pipefail

# ---- versions / locations (override via environment) ------------------------
KICAD_PPA_VERSION="${KICAD_PPA_VERSION:-9.0}"
FABKIT_VERSION="${FABKIT_VERSION:-5.3.0}"
FABKIT_SHA256="${FABKIT_SHA256:-70fe1ecc4858c5c44a3aedb6eafff80515e7837e48921c983c6fb2d8b19d9938}"
FABKIT_ID="com.github.bennymeg.JLC-Plugin-for-KiCad"
FREEROUTING_VERSION="${FREEROUTING_VERSION:-2.4.1}"
EDA_DIR="${EDA_DIR:-/opt/eda}"
KICAD_USER_HOME="${KICAD_USER_HOME:-$HOME}"
KICAD_MM="${KICAD_PPA_VERSION}"                     # e.g. 9.0
KICAD_MAJOR="${KICAD_PPA_VERSION%%.*}"              # e.g. 9
JAVA25="/usr/lib/jvm/java-25-openjdk-amd64/bin/java"

log()  { printf '\n\033[1;34m==> %s\033[0m\n' "$*"; }
ok()   { printf '    \033[32mok\033[0m  %s\n' "$*"; }
die()  { printf '\033[1;31mERROR:\033[0m %s\n' "$*" >&2; exit 1; }

export DEBIAN_FRONTEND=noninteractive

require_env() {
    [ "$(id -u)" -eq 0 ] || die "run as root (sudo ./setup-eda.sh)"
    . /etc/os-release
    [ "${ID:-}" = "ubuntu" ] && [ "${VERSION_ID:-}" = "24.04" ] \
        || die "built for Ubuntu 24.04 (found ${PRETTY_NAME:-unknown})"
    [ "$(uname -m)" = "x86_64" ] || die "built for x86_64 (found $(uname -m))"
}

# ---- 1. KiCad from the official PPA -----------------------------------------
# add-apt-repository is avoided on purpose: it breaks when the default python3
# is not the distro's, so the PPA and its signing key are configured by hand.
install_kicad() {
    log "KiCad ${KICAD_PPA_VERSION} (PCBNew, kicad-cli, libraries)"
    apt-get update -qq
    apt-get install -y -qq curl ca-certificates gnupg unzip python3-venv >/dev/null

    local list="/etc/apt/sources.list.d/kicad-${KICAD_PPA_VERSION}.list"
    local key="/etc/apt/keyrings/kicad-ppa.gpg"
    if [ ! -f "$list" ]; then
        local fp
        fp=$(curl -fsS "https://api.launchpad.net/devel/~kicad/+archive/ubuntu/kicad-${KICAD_PPA_VERSION}-releases" \
            | python3 -c 'import sys,json;print(json.load(sys.stdin)["signing_key_fingerprint"])')
        mkdir -p /etc/apt/keyrings
        curl -fsS "https://keyserver.ubuntu.com/pks/lookup?op=get&search=0x${fp}" | gpg --dearmor > "$key"
        echo "deb [signed-by=$key] https://ppa.launchpadcontent.net/kicad/kicad-${KICAD_PPA_VERSION}-releases/ubuntu noble main" > "$list"
        apt-get update -qq
    fi
    apt-get install -y -qq kicad kicad-libraries kicad-symbols kicad-footprints \
        kicad-packages3d kicad-templates >/dev/null
    ok "$(kicad-cli version)"

    # KiCad normally writes the global library tables on first GUI launch; tools
    # such as SKiDl need them on a headless box, so seed them from the templates.
    local cfg="${KICAD_USER_HOME}/.config/kicad/${KICAD_MM}"
    mkdir -p "$cfg"
    for t in fp-lib-table sym-lib-table; do
        [ -f "$cfg/$t" ] || cp "/usr/share/kicad/template/$t" "$cfg/$t"
    done
    ok "global library tables in $cfg"
}

# ---- 2. SKiDl in a venv that can see KiCad's pcbnew module ------------------
# Must be the distro python (3.12): pcbnew is built for it. --system-site-packages
# exposes pcbnew. A venv is required because pip's own build of SKiDl's
# dependencies fails against the distro-patched setuptools.
install_skidl() {
    log "SKiDl (venv: ${EDA_DIR}/venv)"
    mkdir -p "$EDA_DIR"
    if [ ! -x "${EDA_DIR}/venv/bin/python" ]; then
        /usr/bin/python3.12 -m venv --system-site-packages "${EDA_DIR}/venv"
    fi
    "${EDA_DIR}/venv/bin/pip" install -q --upgrade pip setuptools wheel
    "${EDA_DIR}/venv/bin/pip" install -q skidl
    ok "$("${EDA_DIR}/venv/bin/pip" show skidl | grep -E '^Version')"
}

# ---- 3. Fabrication Toolkit (checksum-verified) -----------------------------
install_fabkit() {
    log "Fabrication Toolkit ${FABKIT_VERSION}"
    mkdir -p "${EDA_DIR}/dl"
    local zip="${EDA_DIR}/dl/Fabrication-Toolkit-${FABKIT_VERSION}.zip"
    if [ ! -f "$zip" ]; then
        curl -fsSL -o "$zip" \
            "https://github.com/bennymeg/Fabrication-Toolkit/releases/download/v${FABKIT_VERSION}/Fabrication-Toolkit-${FABKIT_VERSION}.zip"
    fi
    echo "${FABKIT_SHA256}  ${zip}" | sha256sum -c --quiet - \
        || die "Fabrication Toolkit checksum mismatch — refusing to install"

    local base="${KICAD_USER_HOME}/.local/share/kicad/${KICAD_MM}/3rdparty"
    local tmp; tmp="$(mktemp -d)"
    unzip -q -o "$zip" -d "$tmp"
    mkdir -p "$base/plugins/$FABKIT_ID" "$base/resources/$FABKIT_ID"
    cp -r "$tmp"/plugins/. "$base/plugins/$FABKIT_ID/"
    cp "$tmp/metadata.json" "$base/plugins/$FABKIT_ID/"
    cp -r "$tmp"/resources/. "$base/resources/$FABKIT_ID/"
    rm -r "$tmp"
    ok "plugin in $base/plugins/$FABKIT_ID"
}

# ---- 4. Freerouting + Java 25 ----------------------------------------------
install_freerouting() {
    log "Freerouting ${FREEROUTING_VERSION} + Java 25"
    apt-get install -y -qq openjdk-25-jre-headless >/dev/null
    [ -x "$JAVA25" ] || die "Java 25 not found at $JAVA25"

    local jar="${EDA_DIR}/freerouting-${FREEROUTING_VERSION}.jar"
    if [ ! -f "$jar" ]; then
        curl -fsSL -o "$jar" \
            "https://github.com/freerouting/freerouting/releases/download/v${FREEROUTING_VERSION}/freerouting-${FREEROUTING_VERSION}.jar"
    fi
    unzip -tq "$jar" >/dev/null || die "downloaded Freerouting jar is corrupt"
    ok "$jar"
}

# ---- 5. Environment for every future shell ----------------------------------
write_env() {
    log "Environment (/etc/profile.d/eda.sh)"
    cat > /etc/profile.d/eda.sh <<EOF
# KiCad / SKiDl / Freerouting environment (generated by setup-eda.sh)
export KICAD${KICAD_MAJOR}_SYMBOL_DIR=/usr/share/kicad/symbols
export KICAD${KICAD_MAJOR}_FOOTPRINT_DIR=/usr/share/kicad/footprints
export KICAD${KICAD_MAJOR}_3DMODEL_DIR=/usr/share/kicad/3dmodels
export KICAD_SYMBOL_DIR=/usr/share/kicad/symbols
export FREEROUTING_JAR=${EDA_DIR}/freerouting-${FREEROUTING_VERSION}.jar
export FREEROUTING_JAVA=${JAVA25}
export PATH=${EDA_DIR}/venv/bin:\$PATH
alias freerouting='\$FREEROUTING_JAVA -jar \$FREEROUTING_JAR'
EOF
    ok "wrote /etc/profile.d/eda.sh"
}

# ---- 6. Smoke test: SKiDl -> PCBNew -> DSN -> Freerouting -> SES ------------
smoke_test() {
    log "Smoke test"
    # shellcheck disable=SC1091
    . /etc/profile.d/eda.sh
    local work; work="$(mktemp -d)"
    cd "$work"

    cat > skidl_test.py <<EOF
from skidl import *
set_default_tool(KICAD${KICAD_MAJOR})   # SKiDl defaults to the newest KiCad it knows
r1 = Part("Device", "R", value="1K", footprint="Resistor_SMD:R_0603_1608Metric")
r2 = Part("Device", "R", value="2K", footprint="Resistor_SMD:R_0603_1608Metric")
vin, vout, gnd = Net("VIN"), Net("VOUT"), Net("GND")
vin += r1[1]; vout += r1[2], r2[1]; gnd += r2[2]
generate_netlist()
EOF
    "${EDA_DIR}/venv/bin/python" skidl_test.py >/dev/null 2>&1 \
        && grep -q '(export' skidl_test.net && ok "SKiDl produced a KiCad netlist" \
        || die "SKiDl smoke test failed"

    cat > board.py <<'EOF'
import pcbnew
b = pcbnew.BOARD()
nets = {}
for n in ("VIN", "VOUT", "GND"):
    nets[n] = pcbnew.NETINFO_ITEM(b, n); b.Add(nets[n])
def place(ref, x):
    f = pcbnew.FootprintLoad("/usr/share/kicad/footprints/Resistor_SMD.pretty", "R_0603_1608Metric")
    f.SetReference(ref)
    f.SetPosition(pcbnew.VECTOR2I(pcbnew.FromMM(x), pcbnew.FromMM(20)))
    b.Add(f); return f
r1, r2 = place("R1", 15), place("R2", 30)
r1.FindPadByNumber("1").SetNet(nets["VIN"]);  r1.FindPadByNumber("2").SetNet(nets["VOUT"])
r2.FindPadByNumber("1").SetNet(nets["VOUT"]); r2.FindPadByNumber("2").SetNet(nets["GND"])
pts = [(5, 10), (40, 10), (40, 30), (5, 30), (5, 10)]
for (x1, y1), (x2, y2) in zip(pts, pts[1:]):
    s = pcbnew.PCB_SHAPE(b); s.SetShape(pcbnew.SHAPE_T_SEGMENT); s.SetLayer(pcbnew.Edge_Cuts)
    s.SetStart(pcbnew.VECTOR2I(pcbnew.FromMM(x1), pcbnew.FromMM(y1)))
    s.SetEnd(pcbnew.VECTOR2I(pcbnew.FromMM(x2), pcbnew.FromMM(y2))); b.Add(s)
b.Save("board.kicad_pcb")
assert pcbnew.ExportSpecctraDSN(b, "board.dsn"), "DSN export failed"
EOF
    "${EDA_DIR}/venv/bin/python" board.py && ok "PCBNew built a board and exported DSN"

    timeout 180 "$FREEROUTING_JAVA" -jar "$FREEROUTING_JAR" \
        -de board.dsn -do board.ses -mp 5 --gui.enabled=false >route.log 2>&1 || true
    [ -s board.ses ] && grep -q "COMPLETED" route.log \
        && ok "Freerouting routed the board (board.ses written)" \
        || { tail -20 route.log; die "Freerouting smoke test failed"; }

    ls "${KICAD_USER_HOME}/.local/share/kicad/${KICAD_MM}/3rdparty/plugins/${FABKIT_ID}/plugin.py" >/dev/null \
        && ok "Fabrication Toolkit plugin files present"
    cd / && rm -r "$work"
    printf '\n\033[1;32mAll good.\033[0m Open a new shell (or: source /etc/profile.d/eda.sh).\n'
}

main() {
    case "${1:-}" in
        --test)    require_env; smoke_test ;;
        --no-test) require_env; install_kicad; install_skidl; install_fabkit; install_freerouting; write_env ;;
        "")        require_env; install_kicad; install_skidl; install_fabkit; install_freerouting; write_env; smoke_test ;;
        *)         die "unknown option: $1 (use --test or --no-test)" ;;
    esac
}
main "$@"
