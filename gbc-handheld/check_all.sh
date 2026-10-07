#!/usr/bin/env bash
# Run every check on the routed board, regenerate the fab files, BOM and pictures.
#
#   gbc-handheld/check_all.sh
#
# Prints one line per check; full logs land in build/checks/. Exit code 0 only if all pass.
# Needs the toolchain from ../setup-eda.sh. Downloads Espressif's DevKitC-1 DXF once (not
# committed: it is Espressif's file) for the plug-fit check.
set -uo pipefail
cd "$(dirname "$0")"
# shellcheck disable=SC1091
source /etc/profile.d/eda.sh
PY=/opt/eda/venv/bin/python
B=build
LOG=$B/checks
mkdir -p "$LOG" "$B/ext"
DXF=$B/ext/DXF_ESP32-S3-DevKitC-1_V1.1_20220429.dxf
[ -s "$DXF" ] || curl -fsSL -o "$DXF" \
    https://dl.espressif.com/dl/schematics/esp_idf/DXF_ESP32-S3-DevKitC-1_V1.1_20220429.dxf

status=0
run() {   # run <name> <command...>
    local name=$1; shift
    local f="$LOG/$(echo "$name" | tr ' /' '__').log"
    if "$@" >"$f" 2>&1; then
        printf '  ok    %-28s %s\n' "$name" "$(grep -E 'failure|Found|PASS' "$f" | tail -1)"
    else
        printf '  FAIL  %-28s see %s\n' "$name" "$f"
        grep -E '^FAIL' "$f" | head -5 | sed 's/^/          /'
        status=1
    fi
}

echo "== design checks"
run "ERC (schematic)"        bash -c "grep -q 'No errors or warnings' schematic.erc"
run "netlist check"          $PY -I verify_netlist.py $B/gbc_handheld.net
run "KiCad DRC"              kicad-cli pcb drc --severity-all --all-track-errors --exit-code-violations \
                                 --format report --output $B/drc.rpt $B/gbc_handheld.kicad_pcb
run "board vs netlist"       $PY verify_board.py
run "ESP32 plug-fit (DXF)"   $PY check_plugfit.py "$DXF"
run "3D fit and access"      $PY check_3d.py

echo "== fabrication files"
rm -rf $B/fab && mkdir -p $B/fab
run "export Gerbers"         kicad-cli pcb export gerbers --output $B/fab/ \
                                 --layers F.Cu,B.Cu,F.Mask,B.Mask,F.Silkscreen,B.Silkscreen,Edge.Cuts \
                                 $B/gbc_handheld.kicad_pcb
run "export drill"           kicad-cli pcb export drill --output $B/fab/ --format excellon \
                                 --drill-origin absolute --excellon-units mm --excellon-separate-th \
                                 --generate-report --report-path $B/fab/drill_report.txt $B/gbc_handheld.kicad_pcb
run "zip for JLCPCB"         bash -c "cd $B/fab && rm -f ../gbc_handheld_gerbers.zip && zip -q ../gbc_handheld_gerbers.zip *.g* *.drl"
run "Gerber/drill check"     $PY check_gerbers.py
run "BOM"                    $PY -I make_bom.py $B/gbc_handheld.net
run "paper template"         kicad-cli pcb export pdf --layers F.Cu,B.Cu,Edge.Cuts,F.Silkscreen \
                                 --drill-shape-opt 2 --black-and-white --mode-single \
                                 --output $B/template.pdf $B/placed.kicad_pcb

echo "== pictures"
for side in top bottom; do
    if [ $side = top ]; then L=F.Cu,F.Silkscreen,F.Fab,Edge.Cuts; M=""; else L=B.Cu,B.Silkscreen,B.Fab,Edge.Cuts; M=--mirror; fi
    run "layout $side" bash -c "kicad-cli pcb export svg --layers $L --mode-single --fit-page-to-board \
        --page-size-mode 2 --exclude-drawing-sheet $M --output $B/r_$side.svg $B/gbc_handheld.kicad_pcb \
        && rsvg-convert -w 1300 -b white $B/r_$side.svg -o $B/r_$side.png"
done
for view in top bottom; do
    run "3D render $view" kicad-cli pcb render --side $view --width 1400 --height 1500 --quality high \
        --background opaque --output $B/3d_$view.png $B/gbc_handheld.kicad_pcb
done
run "3D render angled" kicad-cli pcb render --side top --rotate "-35,0,25" --perspective --width 1600 \
    --height 1200 --quality high --background opaque --output $B/3d_angled.png $B/gbc_handheld.kicad_pcb

echo
if [ $status -eq 0 ]; then echo "ALL CHECKS PASSED"; else echo "SOME CHECKS FAILED"; fi
exit $status
