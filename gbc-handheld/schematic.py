#!/usr/bin/env python3
"""Game Boy Color-style handheld, v0.3: schematic as code (SKiDl, KiCad 9 libraries).

ALL THROUGH-HOLE / PLUG-IN: the owner is a beginner and must be able to hand-solder it.

Run (after `source /etc/profile.d/eda.sh`, see ../setup-eda.sh):
    python3 gbc-handheld/schematic.py

Writes build/gbc_handheld.net (KiCad netlist) and build/pinmap.md (GPIO table).
Spec: ../docs/gbc-handheld-spec.md

Sources checked while writing this (links in the spec):
  * ESP32-S3-DevKitC-1 v1.1 header tables J1/J3 (Espressif user guide)
  * MSP2807 datasheet + outline drawing (14-pin header; SD signals are bare pads)
  * MAX98357A breakout behaviour (Adafruit guide): GAIN open = 9 dB; SD low = shutdown,
    1M pull-up to VIN on the breakout = (L+R)/2 mono mix; VIN 2.7-5.5 V
Items marked VERIFY need the real parts / final listings before ordering.

v0.3 changes (from the full audit, spec section 15):
  * slide switch is now the SS-12D00G3 (0.5 A) with a project footprint (gbc.pretty); it only
    switches the LCD. The amplifier runs from USB 5 V directly and is muted by the ESP32.
  * a 15k/22k divider lets the ESP32 read the switch (PWR_SENSE): 5 V -> 2.97 V.
  * backlight is driven by GPIO42 through 1k (dimmable, and safe even if the LED pin turns out
    to drive the LEDs directly); the backlight jumper is gone.
  * LCD SDO (MISO) is left unconnected so it can never fight the SD card on the shared bus.
  * 100 uF bulk capacitor at the amplifier's supply.
"""
import os
from skidl import *  # noqa: F401,F403

set_default_tool(KICAD9)  # SKiDl defaults to KiCad 10; we target 9.

OUT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "build")
os.makedirs(OUT, exist_ok=True)

# --------------------------------------------------------------------------
# Footprints (all verified to exist in the KiCad 9 library). No SMD anywhere.
# --------------------------------------------------------------------------
FP_SOCKET22 = "Connector_PinSocket_2.54mm:PinSocket_1x22_P2.54mm_Vertical"
FP_SOCKET14 = "Connector_PinSocket_2.54mm:PinSocket_1x14_P2.54mm_Vertical"
FP_SOCKET7 = "Connector_PinSocket_2.54mm:PinSocket_1x07_P2.54mm_Vertical"
FP_HDR4 = "Connector_PinHeader_2.54mm:PinHeader_1x04_P2.54mm_Vertical"
FP_TACT = "Button_Switch_THT:SW_PUSH_6mm"
FP_SLIDE = "gbc:SW_Slide_SS12D00G3"          # project footprint, see gen_footprints.py
FP_R = "Resistor_THT:R_Axial_DIN0207_L6.3mm_D2.5mm_P7.62mm_Horizontal"
FP_C_DISC = "Capacitor_THT:C_Disc_D5.0mm_W2.5mm_P5.00mm"
FP_C_ELEC = "Capacitor_THT:CP_Radial_D5.0mm_P2.50mm"   # 2.5 mm pitch: 0.9 mm pad gap, beginner-safe
FP_LED = "LED_THT:LED_D3.0mm"

# --------------------------------------------------------------------------
# Nets
# --------------------------------------------------------------------------
gnd = Net("GND")
v3v3 = Net("+3V3")        # from the dev board's 3.3 V regulator
v5_usb = Net("+5V_USB")   # dev board 5 V pin (USB VBUS)
v5_sw = Net("+5V_SW")     # after the power slide switch: LCD + amplifier only
for n in (gnd, v3v3, v5_usb, v5_sw):
    n.drive = POWER

# --------------------------------------------------------------------------
# ESP32-S3-DevKitC-1 headers (verified against Espressif's J1/J3 tables)
# --------------------------------------------------------------------------
J1_MAP = {1: "3V3", 2: "3V3", 3: "RST", 4: "GPIO4", 5: "GPIO5", 6: "GPIO6",
          7: "GPIO7", 8: "GPIO15", 9: "GPIO16", 10: "GPIO17", 11: "GPIO18",
          12: "GPIO8", 13: "GPIO3", 14: "GPIO46", 15: "GPIO9", 16: "GPIO10",
          17: "GPIO11", 18: "GPIO12", 19: "GPIO13", 20: "GPIO14", 21: "5V",
          22: "GND"}
J3_MAP = {1: "GND", 2: "GPIO43", 3: "GPIO44", 4: "GPIO1", 5: "GPIO2",
          6: "GPIO42", 7: "GPIO41", 8: "GPIO40", 9: "GPIO39", 10: "GPIO38",
          11: "GPIO37", 12: "GPIO36", 13: "GPIO35", 14: "GPIO0", 15: "GPIO45",
          16: "GPIO48", 17: "GPIO47", 18: "GPIO21", 19: "GPIO20", 20: "GPIO19",
          21: "GND", 22: "GND"}

j1 = Part("Connector_Generic", "Conn_01x22", ref="J1", footprint=FP_SOCKET22,
          value="ESP32-S3-DevKitC-1 J1")
j3 = Part("Connector_Generic", "Conn_01x22", ref="J3", footprint=FP_SOCKET22,
          value="ESP32-S3-DevKitC-1 J3")

# Pins we must never use for a signal on an ESP32-S3 N16R8 DevKitC-1:
FORBIDDEN = (
    {0, 3, 45, 46}              # strapping pins
    | {19, 20}                  # native USB D-/D+
    | {43, 44}                  # UART0 (USB-serial bridge)
    | set(range(26, 38))        # flash + octal PSRAM (26-32, 33-37)
    | {38}                      # on-board RGB LED (v1.1)
    | {47, 48}                  # also flash clock lines on some modules
)

_gpio_pin = {}
for hdr, mp in ((j1, J1_MAP), (j3, J3_MAP)):
    for num, fn in mp.items():
        if fn.startswith("GPIO"):
            _gpio_pin[int(fn[4:])] = hdr[num]


def gpio(n, net_name):
    """Wire ESP32 GPIO n to a named net, refusing forbidden pins."""
    assert n not in FORBIDDEN, f"GPIO{n} is forbidden on the N16R8 DevKitC-1"
    net = Net(net_name)
    net += _gpio_pin[n]
    return net


for hdr, mp in ((j1, J1_MAP), (j3, J3_MAP)):
    for num, fn in mp.items():
        if fn == "3V3":
            v3v3 += hdr[num]
        elif fn == "5V":
            v5_usb += hdr[num]
        elif fn == "GND":
            gnd += hdr[num]
j1[3] += NC  # RST: not used (the dev board has its own RST button)


def mark_unused_header_pins():
    """Every header pin we did not wire is intentionally unconnected."""
    for hdr in (j1, j3):
        for pin in hdr.pins:
            if not pin.is_connected():
                pin += NC


# --------------------------------------------------------------------------
# GPIO assignment (see build/pinmap.md for the generated table)
# --------------------------------------------------------------------------
GPIO_USED = {}


def assign(n, name):
    GPIO_USED[n] = name
    return gpio(n, name)


# Buttons: switch between the GPIO and GND, internal pull-up in firmware.
BUTTONS = [("UP", 1), ("DOWN", 2), ("LEFT", 4), ("RIGHT", 5),
           ("A", 6), ("B", 7), ("START", 15), ("SELECT", 16)]
for i, (label, g) in enumerate(BUTTONS, start=1):
    sw = Part("Switch", "SW_Push", ref=f"SW{i}", footprint=FP_TACT,
              value=f"SW_{label}")
    sw[1] += assign(g, f"BTN_{label}")
    sw[2] += gnd

# Shared SPI bus for the LCD and the SD card (FSPI default IO_MUX pins: 10-13)
lcd_cs = assign(10, "LCD_CS")
spi_mosi = assign(11, "SPI_MOSI")
spi_sck = assign(12, "SPI_SCK")
spi_miso = assign(13, "SPI_MISO")      # only the SD card drives this line
lcd_bl = assign(42, "LCD_BL")          # backlight on/off and PWM dimming
pwr_sense = assign(39, "PWR_SENSE")    # reads the slide switch through a divider
lcd_dc = assign(9, "LCD_DC")
lcd_rst = assign(8, "LCD_RST")
sd_cs = assign(21, "SD_CS")
status_led = assign(14, "STATUS_LED")
# I2S audio to the MAX98357A module. Any free GPIO works through the ESP32-S3 GPIO matrix.
audio_bclk = assign(17, "AUDIO_BCLK")
audio_lrc = assign(18, "AUDIO_LRC")
audio_din = assign(40, "AUDIO_DIN")
audio_mute = assign(41, "AUDIO_SD")

# --------------------------------------------------------------------------
# LCD module socket: MSP2807, 14-pin header (verified from the datasheet and the
# manufacturer's outline drawing). The module's back faces our board.
# 1 VCC 2 GND 3 CS 4 RESET 5 DC 6 SDI(MOSI) 7 SCK 8 LED 9 SDO(MISO)
# 10 T_CLK 11 T_CS 12 T_DIN 13 T_DO 14 T_IRQ  (touch: not used)
# --------------------------------------------------------------------------
j_lcd = Part("Connector_Generic", "Conn_01x14", ref="J_LCD",
             footprint=FP_SOCKET14, value="2.8in ILI9341 (MSP2807)")
j_lcd[1] += v5_sw
j_lcd[2] += gnd
j_lcd[3] += lcd_cs
j_lcd[4] += lcd_rst
j_lcd[5] += lcd_dc
j_lcd[6] += spi_mosi
j_lcd[7] += spi_sck
# Pin 9 SDO (MISO) left open: the datasheet allows it ("if you do not need the read function,
# you can not connect it"), and it keeps the display off the SD card's MISO line.
j_lcd[9] += NC
for pin in (10, 11, 12, 13, 14):
    j_lcd[pin] += NC

# Backlight: LED pin = "backlight control, high level lighting" (datasheet). Driven from
# GPIO42 through 1k: plenty for a transistor input, and it limits the GPIO current to about
# 3 mA even if a module feeds its LEDs straight from this pin.
r_bl = Part("Device", "R", ref="R_BL", value="1k", footprint=FP_R)
r_bl[1] += lcd_bl
r_bl[2] += j_lcd[8]

# SD slot: the module's SD interface (SD_CS, SD_MOSI, SD_MISO, SD_SCK) is four bare
# round SOLDER PADS on the module's back, NOT a header. The owner solders four short
# wires from those pads to this 4-pin header. Pin order = module silkscreen read
# bottom to top: SD_CS, SD_MOSI, SD_MISO, SD_SCK. VERIFY on the real module.
j_sd = Part("Connector_Generic", "Conn_01x04", ref="J_SD",
            footprint=FP_HDR4, value="SD wires: CS MOSI MISO SCK")
j_sd[1] += sd_cs
j_sd[2] += spi_mosi
j_sd[3] += spi_miso
j_sd[4] += spi_sck

# --------------------------------------------------------------------------
# Power switch: SS-12D00G3 slide (0.5 A). Common = USB 5 V, throw = switched 5 V for the LCD
# only (about 0.1 A). It does NOT power the ESP32 or the amplifier off; the ESP32 reads the
# switch through PWR_SENSE and mutes the amplifier / sleeps in firmware.
# --------------------------------------------------------------------------
sw_pwr = Part("Switch", "SW_SPDT", ref="SW_PWR", footprint=FP_SLIDE,
              value="Power (slide)")
sw_pwr[2] += v5_usb
sw_pwr[1] += v5_sw
sw_pwr[3] += NC

# Switch sense divider: +5V_SW -> 15k -> PWR_SENSE -> 22k -> GND.
# 5.00 V -> 2.97 V; worst case USB 5.25 V -> 3.12 V (below the 3.6 V limit),
# 4.75 V -> 2.82 V (above the 2.48 V logic-high threshold at 3.3 V).
r_s1 = Part("Device", "R", ref="R_S1", value="15k", footprint=FP_R)
r_s2 = Part("Device", "R", ref="R_S2", value="22k", footprint=FP_R)
r_s1[1] += v5_sw
r_s1[2] += pwr_sense
r_s2[1] += pwr_sense
r_s2[2] += gnd

# Bulk (electrolytic, mind the polarity) + high-frequency decoupling on the switched rail
c_bulk = Part("Device", "C_Polarized", ref="C_BULK", value="100uF",
              footprint=FP_C_ELEC)
c_bulk[1] += v5_sw     # + side
c_bulk[2] += gnd       # - side (stripe)
c_hf = Part("Device", "C", ref="C_HF", value="100nF", footprint=FP_C_DISC)
c_hf[1] += v5_sw
c_hf[2] += gnd

# --------------------------------------------------------------------------
# Status LED on GPIO14 (3 mm, through-hole)
# --------------------------------------------------------------------------
r_led = Part("Device", "R", ref="R_LED", value="1k", footprint=FP_R)
d_led = Part("Device", "LED", ref="D_STATUS", value="LED 3mm", footprint=FP_LED)
r_led[1] += status_led
r_led[2] += d_led[2]   # anode (KiCad LED symbol: 1=K, 2=A)
d_led[1] += gnd

# --------------------------------------------------------------------------
# Audio: MAX98357A I2S amplifier module on a 7-pin socket. The module's own screw
# terminal or pads carry the speaker; no audio parts are needed on our board.
# Header order assumed from the Adafruit breakout silkscreen, left to right with the
# module front facing up:  LRC, BCLK, DIN, GAIN, SD, GND, VIN.
# VERIFY against the exact module listing: clones sometimes reverse the order. If so,
# reverse AMP_ORDER here and re-run; nothing else changes.
# GAIN left open = 9 dB. SD: the module pulls it to VIN (mono mix); the ESP32 can pull
# it low to mute (firmware: drive low, or high-Z for normal).
# --------------------------------------------------------------------------
AMP_ORDER = ["LRC", "BCLK", "DIN", "GAIN", "SD", "GND", "VIN"]
j_amp = Part("Connector_Generic", "Conn_01x07", ref="J_AMP",
             footprint=FP_SOCKET7, value="MAX98357A: " + " ".join(AMP_ORDER))
# VIN from USB 5 V directly: the amp can draw ~0.6 A peaks, more than a mini slide switch.
amp_nets = {"LRC": audio_lrc, "BCLK": audio_bclk, "DIN": audio_din,
            "SD": audio_mute, "GND": gnd, "VIN": v5_usb}
for idx, name in enumerate(AMP_ORDER, start=1):
    if name == "GAIN":
        j_amp[idx] += NC
    else:
        j_amp[idx] += amp_nets[name]

# Bulk capacitor at the amplifier supply (class-D current peaks), electrolytic: mind polarity.
c_amp = Part("Device", "C_Polarized", ref="C_AMP", value="100uF", footprint=FP_C_ELEC)
c_amp[1] += v5_usb
c_amp[2] += gnd


# --------------------------------------------------------------------------
# Outputs
# --------------------------------------------------------------------------
def write_pinmap():
    lines = ["# ESP32-S3-DevKitC-1 pin map (generated by schematic.py)\n",
             "| GPIO | Signal |", "|---|---|"]
    for n in sorted(GPIO_USED):
        lines.append(f"| GPIO{n} | {GPIO_USED[n]} |")
    lines += ["", "Forbidden on this board: " +
              ", ".join(f"GPIO{n}" for n in sorted(FORBIDDEN)) + "."]
    with open(os.path.join(OUT, "pinmap.md"), "w") as f:
        f.write("\n".join(lines) + "\n")


if __name__ == "__main__":
    mark_unused_header_pins()
    write_pinmap()
    ERC()
    generate_netlist(file_=os.path.join(OUT, "gbc_handheld.net"))
    print("netlist written to", os.path.join(OUT, "gbc_handheld.net"))
