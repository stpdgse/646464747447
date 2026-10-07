#!/usr/bin/env python3
"""Game Boy Color-style handheld: schematic as code (SKiDl, KiCad 9 libraries).

Run (after `source /etc/profile.d/eda.sh`, see ../setup-eda.sh):
    python3 gbc-handheld/schematic.py

Writes build/gbc_handheld.net (KiCad netlist), build/gbc_handheld.erc, and
build/pinmap.md (ESP32 header assignment). Spec: ../docs/gbc-handheld-spec.md

Sources checked while writing this (see the spec for links):
  * ESP32-S3-DevKitC-1 v1.1 header tables J1/J3 (Espressif user guide)
  * PAM8302A datasheet (Diodes Inc., Rev 2-5): SOP-8 pinout, typical circuit,
    gain A = 20*log[2*(RF/RI)], RI(min) = 10k internal, RF = 80k
  * MSP2807 datasheet (14-pin header, no SD pins listed there)
Items still marked VERIFY need the real parts in hand or a closer look.
"""
import os
from skidl import *  # noqa: F401,F403

set_default_tool(KICAD9)  # SKiDl defaults to KiCad 10; we target 9.

OUT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "build")
os.makedirs(OUT, exist_ok=True)

# --------------------------------------------------------------------------
# Footprints (all verified to exist in the KiCad 9 library)
# --------------------------------------------------------------------------
FP_SOCKET22 = "Connector_PinSocket_2.54mm:PinSocket_1x22_P2.54mm_Vertical"
FP_SOCKET14 = "Connector_PinSocket_2.54mm:PinSocket_1x14_P2.54mm_Vertical"
FP_SOCKET4 = "Connector_PinSocket_2.54mm:PinSocket_1x04_P2.54mm_Vertical"
FP_HDR2 = "Connector_PinHeader_2.54mm:PinHeader_1x02_P2.54mm_Vertical"
FP_TACT = "Button_Switch_THT:SW_PUSH_6mm"
FP_SLIDE = "Button_Switch_THT:SW_Slide_SPDT_Straight_CK_OS102011MS2Q"
FP_SOIC8 = "Package_SO:SOIC-8_3.9x4.9mm_P1.27mm"  # PAM8302A *ADCR* (SOP-8)
FP_R = "Resistor_SMD:R_0805_2012Metric"
FP_C = "Capacitor_SMD:C_0805_2012Metric"
FP_LED = "LED_SMD:LED_0805_2012Metric"

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
# Pin number -> function. Numbers are header positions 1..22.
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


# Power / ground / unused pins on the ESP32 headers
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

# LCD (SPI2/FSPI default "IO_MUX" pins: CS=10, MOSI=11, SCK=12, MISO=13)
lcd_cs = assign(10, "LCD_CS")
lcd_mosi = assign(11, "LCD_MOSI")
lcd_sck = assign(12, "LCD_SCK")
lcd_miso = assign(13, "LCD_MISO")
lcd_dc = assign(9, "LCD_DC")
lcd_rst = assign(8, "LCD_RST")
sd_cs = assign(21, "SD_CS")
audio_pwm = assign(17, "AUDIO_PWM")
status_led = assign(14, "STATUS_LED")
RESERVED = {18: "spare (future L button / second status)"}

# --------------------------------------------------------------------------
# LCD module socket: MSP2807, 14-pin header (verified from the datasheet)
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
j_lcd[6] += lcd_mosi
j_lcd[7] += lcd_sck
j_lcd[9] += lcd_miso
# Backlight: datasheet says tie to 3.3 V for always-on. Series 0R so it can be
# changed to a GPIO or removed. VERIFY against the chosen module listing.
r_bl = Part("Device", "R", ref="R_BL", value="0R", footprint=FP_R)
r_bl[1] += v3v3
r_bl[2] += j_lcd[8]
for pin in (10, 11, 12, 13, 14):
    j_lcd[pin] += NC

# SD slot: the MSP2807 datasheet lists no SD pins. Many modules put the slot on
# a separate 4-pin header (SD_CS, SD_MOSI, SD_MISO, SD_SCK). PROVISIONAL.
# VERIFY the pin order against the exact module listing before ordering.
j_sd = Part("Connector_Generic", "Conn_01x04", ref="J_SD",
            footprint=FP_SOCKET4, value="SD pins (provisional)")
j_sd[1] += sd_cs
j_sd[2] += lcd_mosi
j_sd[3] += lcd_miso
j_sd[4] += lcd_sck

# --------------------------------------------------------------------------
# Power switch: SPDT slide. Common = USB 5 V, throw A = switched 5 V.
# Cuts power to the LCD and amplifier. It does NOT power the ESP32 off.
# --------------------------------------------------------------------------
sw_pwr = Part("Switch", "SW_SPDT", ref="SW_PWR", footprint=FP_SLIDE,
              value="Power (slide)")
sw_pwr[2] += v5_usb
sw_pwr[1] += v5_sw
sw_pwr[3] += NC

# Bulk + local decoupling for the switched rail
c_bulk = Part("Device", "C", ref="C_BULK", value="10uF", footprint=FP_C)
c_hf = Part("Device", "C", ref="C_HF", value="100nF", footprint=FP_C)
for c in (c_bulk, c_hf):
    c[1] += v5_sw
    c[2] += gnd

# --------------------------------------------------------------------------
# Status LED on GPIO14
# --------------------------------------------------------------------------
r_led = Part("Device", "R", ref="R_LED", value="1k", footprint=FP_R)
d_led = Part("Device", "LED", ref="D_STATUS", value="LED", footprint=FP_LED)
r_led[1] += status_led
r_led[2] += d_led[2]   # anode (KiCad LED symbol: 1=K, 2=A)
d_led[1] += gnd

# --------------------------------------------------------------------------
# Audio: PWM -> RC low-pass -> PAM8302A (SOP-8) -> speaker header
# Pin order VERIFIED from the datasheet: 1 /SD, 2 NC, 3 IN+, 4 IN-, 5 VO+,
# 6 VDD, 7 GND, 8 VO-.  BUY: PAM8302AADCR (SOP-8). The AASCR is MSOP-8.
# --------------------------------------------------------------------------
pam8302a = Part(
    tool=SKIDL, name="PAM8302A", dest=TEMPLATE, ref_prefix="U",
    pins=[
        Pin(num=1, name="~{SD}", func=Pin.types.INPUT),
        Pin(num=2, name="NC", func=Pin.types.NOCONNECT),
        Pin(num=3, name="IN+", func=Pin.types.INPUT),
        Pin(num=4, name="IN-", func=Pin.types.INPUT),
        Pin(num=5, name="VO+", func=Pin.types.OUTPUT),
        Pin(num=6, name="VDD", func=Pin.types.PWRIN),
        Pin(num=7, name="GND", func=Pin.types.PWRIN),
        Pin(num=8, name="VO-", func=Pin.types.OUTPUT),
    ],
)
u_amp = pam8302a(ref="U_AMP", footprint=FP_SOIC8, value="PAM8302AADCR")
u_amp[2] += NC
u_amp[6] += v5_sw
u_amp[7] += gnd

# Supply decoupling per datasheet: 1uF close to VDD plus 10uF
c_amp1 = Part("Device", "C", ref="C_AMP1", value="1uF", footprint=FP_C)
c_amp10 = Part("Device", "C", ref="C_AMP10", value="10uF", footprint=FP_C)
for c in (c_amp1, c_amp10):
    c[1] += v5_sw
    c[2] += gnd

# Always-on: pull /SD high. (A GPIO mute can replace this later.)
r_sd = Part("Device", "R", ref="R_SD", value="100k", footprint=FP_R)
r_sd[1] += v5_sw
r_sd[2] += u_amp[1]

# PWM low-pass: 1k + 10nF, corner ~16 kHz
r_f = Part("Device", "R", ref="R_F", value="1k", footprint=FP_R)
c_f = Part("Device", "C", ref="C_F", value="10nF", footprint=FP_C)
audio_filt = Net("AUDIO_FILT")
r_f[1] += audio_pwm
r_f[2] += audio_filt
c_f[1] += audio_filt
c_f[2] += gnd

# Input network, symmetrical (datasheet typical circuit): Ci then R, per input.
# Ci = 0.22uF -> high-pass corner ~72 Hz with RI=10k.
# External R = 22k sets gain A = 20*log(2*80k/(10k+22k)) = ~14 dB.
# Keep the PWM amplitude low in firmware to avoid clipping into 5 V.
c_inp = Part("Device", "C", ref="C_INP", value="220nF", footprint=FP_C)
r_inp = Part("Device", "R", ref="R_INP", value="22k", footprint=FP_R)
c_inn = Part("Device", "C", ref="C_INN", value="220nF", footprint=FP_C)
r_inn = Part("Device", "R", ref="R_INN", value="22k", footprint=FP_R)
amp_inp, amp_inn = Net("AMP_IN_P"), Net("AMP_IN_N")
audio_filt += c_inp[1]
c_inp[2] += r_inp[1]
amp_inp += r_inp[2], u_amp[3]
gnd += c_inn[1]
c_inn[2] += r_inn[1]
amp_inn += r_inn[2], u_amp[4]

j_spk = Part("Connector_Generic", "Conn_01x02", ref="J_SPK",
             footprint=FP_HDR2, value="Speaker 8R")
j_spk[1] += u_amp[5]   # VO+
j_spk[2] += u_amp[8]   # VO-


# --------------------------------------------------------------------------
# Outputs
# --------------------------------------------------------------------------
def write_pinmap():
    lines = ["# ESP32-S3-DevKitC-1 pin map (generated by schematic.py)\n",
             "| GPIO | Signal |", "|---|---|"]
    for n in sorted(GPIO_USED):
        lines.append(f"| GPIO{n} | {GPIO_USED[n]} |")
    for n, why in sorted(RESERVED.items()):
        lines.append(f"| GPIO{n} | reserved: {why} |")
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
