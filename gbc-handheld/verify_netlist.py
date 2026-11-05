#!/usr/bin/env python3
"""Independent check of build/gbc_handheld.net (design v0.3, all through-hole).

Parses the KiCad netlist file itself with a small s-expression reader (not the
SKiDl objects that wrote it) and asserts the connections that would wreck a
board if wrong. Exit code 0 only if every check passes.

Run:  python3 -I gbc-handheld/verify_netlist.py gbc-handheld/build/gbc_handheld.net
"""
import re
import sys


def read_sexpr(text):
    """Parse KiCad s-expression text into nested lists of strings."""
    tokens = re.findall(r'\(|\)|"(?:[^"\\]|\\.)*"|[^\s()"]+', text)
    pos = 0

    def parse():
        nonlocal pos
        tok = tokens[pos]
        pos += 1
        if tok == "(":
            out = []
            while tokens[pos] != ")":
                out.append(parse())
            pos += 1
            return out
        if tok.startswith('"'):
            return tok[1:-1].replace('\\"', '"')
        return tok

    return parse()


def find_all(node, key):
    return [c for c in node if isinstance(c, list) and c and c[0] == key]


def field(node, key):
    for c in node:
        if isinstance(c, list) and c and c[0] == key and len(c) > 1:
            return c[1]
    return None


def load(path):
    tree = read_sexpr(open(path).read())
    comps, nets = {}, {}
    for section in find_all(tree, "components"):
        for comp in find_all(section, "comp"):
            comps[field(comp, "ref")] = {"value": field(comp, "value"),
                                         "fp": field(comp, "footprint")}
    for section in find_all(tree, "nets"):
        for net in find_all(section, "net"):
            nodes = {(field(n, "ref"), field(n, "pin"))
                     for n in find_all(net, "node")}
            nets[field(net, "name")] = nodes
    return comps, nets


def main(path):
    comps, nets = load(path)
    fails = []

    def check(cond, msg):
        print(("PASS  " if cond else "FAIL  ") + msg)
        if not cond:
            fails.append(msg)

    def net_of(ref, pin):
        for name, nodes in nets.items():
            if (ref, str(pin)) in nodes:
                return name
        return None

    def same(a, b):
        """Two pins share one net. Both must exist: None never matches."""
        na, nb = net_of(*a), net_of(*b)
        return na is not None and na == nb

    def is_nc(ref, pin):
        n = net_of(ref, pin)
        return n is None or n.lower().startswith("unconnected")

    # 5 connectors + 9 switches + 4 resistors + 3 capacitors + 1 LED
    check(len(comps) == 22, f"22 parts expected, found {len(comps)}")
    check(len(nets) >= 20, f"{len(nets)} nets parsed")
    check(all(c["fp"] for c in comps.values()), "every part has a footprint")
    # Beginner requirement: no surface-mount footprints at all
    smd = [r for r, c in comps.items() if re.search(r"(?i)_SMD|SOIC|SOP|QFN|0805|0603|0402",
                                                    c["fp"] or "")]
    check(not smd, f"no SMD footprints (found: {smd})")

    # MAX98357A module header, assumed Adafruit order (VERIFY): LRC BCLK DIN GAIN SD GND VIN
    amp = {1: "AUDIO_LRC", 2: "AUDIO_BCLK", 3: "AUDIO_DIN", 5: "AUDIO_SD",
           6: "GND", 7: "+5V_USB"}
    for pin, want in amp.items():
        check(net_of("J_AMP", pin) == want, f"J_AMP pin {pin} on {want}")
    check(is_nc("J_AMP", 4), "J_AMP pin 4 (GAIN) left open = 9 dB")

    # LCD header (MSP2807): 1 VCC 2 GND 3 CS 4 RST 5 DC 6 MOSI 7 SCK 8 LED 9 MISO
    expect = {1: "+5V_SW", 2: "GND", 3: "LCD_CS", 4: "LCD_RST", 5: "LCD_DC",
              6: "SPI_MOSI", 7: "SPI_SCK"}
    for pin, want in expect.items():
        check(net_of("J_LCD", pin) == want, f"LCD pin {pin} on {want}")
    check(is_nc("J_LCD", 9), "LCD pin 9 (SDO/MISO) left open: cannot fight the SD card")
    check(net_of("R_BL", 1) == "LCD_BL" and same(("R_BL", 2), ("J_LCD", 8))
          and comps["R_BL"]["value"] == "1k", "backlight: GPIO net LCD_BL -> 1k -> LCD LED pin")
    for pin in (10, 11, 12, 13, 14):
        check(is_nc("J_LCD", pin), f"LCD touch pin {pin} not connected")

    # SD wires share the LCD SPI bus
    check(net_of("J_SD", 1) == "SD_CS" and net_of("J_SD", 2) == "SPI_MOSI"
          and net_of("J_SD", 3) == "SPI_MISO" and net_of("J_SD", 4) == "SPI_SCK",
          "SD header: CS own pin, MOSI/MISO/SCK on the shared bus")
    check(nets.get("SPI_MISO") and {r for r, _ in nets["SPI_MISO"]} == {"J1", "J_SD"},
          "SPI_MISO connects only the ESP32 and the SD card")

    # Power switch must not short USB 5V to the switched rail
    check(net_of("SW_PWR", 2) == "+5V_USB" and net_of("SW_PWR", 1) == "+5V_SW",
          "power switch: common=+5V_USB, throw=+5V_SW")
    check(is_nc("SW_PWR", 3), "power switch third pin unused")
    check(comps["SW_PWR"]["fp"] == "gbc:SW_Slide_SS12D00G3", "power switch footprint is the SS-12D00G3")
    check({r for r, _ in nets["+5V_SW"]} == {"SW_PWR", "J_LCD", "C_BULK", "C_HF", "R_S1"},
          "switched 5 V feeds only the LCD (+ its capacitors and the sense divider)")

    # Switch-sense divider: values read from the netlist, voltage range computed
    check(net_of("R_S1", 1) == "+5V_SW" and net_of("R_S1", 2) == "PWR_SENSE"
          and net_of("R_S2", 1) == "PWR_SENSE" and net_of("R_S2", 2) == "GND",
          "sense divider: +5V_SW -> R_S1 -> PWR_SENSE -> R_S2 -> GND")

    def ohms(v):
        return float(v.lower().replace("k", "e3").replace("m", "e6"))
    r1, r2 = ohms(comps["R_S1"]["value"]), ohms(comps["R_S2"]["value"])
    vmax, vmin = 5.25 * r2 / (r1 + r2), 4.75 * r2 / (r1 + r2)
    check(vmax < 3.6 and vmin > 0.75 * 3.3,
          f"PWR_SENSE stays within 3.3 V logic: {vmin:.2f}-{vmax:.2f} V for USB 4.75-5.25 V")

    # Dev board power pins: J1 pin 1/2 = 3V3, 21 = 5V, 22 = GND
    check(net_of("J1", 1) == "+3V3" and net_of("J1", 2) == "+3V3",
          "J1 pins 1,2 on +3V3")
    check(net_of("J1", 21) == "+5V_USB", "J1 pin 21 (5V) on +5V_USB")
    check(net_of("J1", 22) == "GND" and net_of("J3", 1) == "GND",
          "ground pins on GND")
    check(len({"GND", "+3V3", "+5V_USB", "+5V_SW"} & set(nets)) == 4,
          "all four power nets exist")

    # Capacitors: electrolytic + side on the switched 5 V rail
    check(net_of("C_BULK", 1) == "+5V_SW" and net_of("C_BULK", 2) == "GND",
          "bulk electrolytic: + on +5V_SW, - on GND")
    check(net_of("C_HF", 1) == "+5V_SW" and net_of("C_HF", 2) == "GND",
          "100nF across +5V_SW and GND")
    check(net_of("C_AMP", 1) == "+5V_USB" and net_of("C_AMP", 2) == "GND",
          "amplifier bulk electrolytic: + on +5V_USB, - on GND")

    # Status LED
    check(net_of("R_LED", 1) == "STATUS_LED" and same(("R_LED", 2), ("D_STATUS", 2))
          and net_of("D_STATUS", 1) == "GND",
          "status LED: GPIO net -> resistor -> anode, cathode on GND")

    # Eight buttons, each from its own GPIO net to GND
    seen = set()
    for i in range(1, 9):
        a, b = net_of(f"SW{i}", 1), net_of(f"SW{i}", 2)
        check(b == "GND" and a is not None and a.startswith("BTN_"),
              f"SW{i}: {a} -> GND")
        seen.add(a)
    check(len(seen) == 8, "eight distinct button nets")

    # GPIO pin positions vs Espressif's tables (J1/J3 pin number -> GPIO)
    positions = {("J1", 4): "BTN_LEFT", ("J1", 5): "BTN_RIGHT",
                 ("J1", 6): "BTN_A", ("J1", 7): "BTN_B", ("J1", 8): "BTN_START",
                 ("J1", 9): "BTN_SELECT", ("J1", 10): "AUDIO_BCLK",
                 ("J1", 11): "AUDIO_LRC", ("J1", 12): "LCD_RST", ("J1", 15): "LCD_DC",
                 ("J1", 16): "LCD_CS", ("J1", 17): "SPI_MOSI", ("J1", 18): "SPI_SCK",
                 ("J1", 19): "SPI_MISO", ("J1", 20): "STATUS_LED",
                 ("J3", 6): "LCD_BL", ("J3", 9): "PWR_SENSE",
                 ("J3", 4): "BTN_UP", ("J3", 5): "BTN_DOWN",
                 ("J3", 7): "AUDIO_SD", ("J3", 8): "AUDIO_DIN", ("J3", 18): "SD_CS"}
    for (ref, pin), want in positions.items():
        check(net_of(ref, pin) == want, f"{ref} pin {pin} carries {want}")

    # Forbidden header positions must stay unconnected
    # (J1 3=RST 13=GPIO3 14=GPIO46; J3 2,3=UART0 10=GPIO38 11-13=PSRAM
    #  14=GPIO0 15=GPIO45 16,17=GPIO48/47 19,20=USB)
    forbidden_pos = [("J1", 3), ("J1", 13), ("J1", 14), ("J3", 2), ("J3", 3),
                     ("J3", 10), ("J3", 11), ("J3", 12), ("J3", 13), ("J3", 14),
                     ("J3", 15), ("J3", 16), ("J3", 17), ("J3", 19), ("J3", 20)]
    for ref, pin in forbidden_pos:
        check(is_nc(ref, pin), f"{ref} pin {pin} left unconnected (got {net_of(ref, pin)})")

    # No signal net may dangle with a single node (power and no-connects aside)
    skip = {"GND", "+3V3", "+5V_USB", "+5V_SW"}
    for name, nodes in nets.items():
        if name in skip or name.lower().startswith("unconnected"):
            continue
        check(len(nodes) >= 2, f"net {name} has {len(nodes)} nodes")

    print(f"\n{len(fails)} failure(s)")
    return 1 if fails else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1]))
