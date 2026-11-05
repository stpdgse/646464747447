# Game Boy Color-style handheld — project spec (v0.1, draft)

Owner: Tom Pearl. Status: **requirements gathered, no design started.**
This is a practice board: the goal is to learn ordering a PCB and soldering it, ending
with something fun. Anything marked **VERIFY** is from memory or a web search and must be
checked against a datasheet or the real part before the design is frozen.

## 1. Goals and non-goals

Goals
- A portrait, classic-Game-Boy-shaped handheld that plays Game Boy and Game Boy Color games.
- A first-ever PCB for the owner: forgiving, hand-solderable, cheap.
- Open, reproducible design: SKiDl schematic -> PCBNew layout -> Freerouting -> DRC (see `setup-eda.sh`).

Non-goals (for this board)
- GBA emulation. The ESP32-S3 is too slow for it (about 20 fps with frameskip reported
  for the 44vba emulator on an ESP32-S3). Leaves L/R as an optional future footprint only.
- Battery power, a case, wireless features, FPGA/BGA parts. Those belong to later projects.

## 2. Decisions so far (from the question rounds)

| Area | Decision |
|---|---|
| Approach | "Mix": own ESP32-S3 board plugs into sockets; the PCB carries real components of its own |
| Brain | Owner's **ESP32-S3-DevKitC-1 (N16R8)** dev board (16 MB flash, 8 MB octal PSRAM), plugged into female headers |
| Screen | Ready-made 2.8" 240x320 SPI LCD module (ILI9341-class) on header pins |
| Game storage | The LCD module's built-in microSD slot |
| Emulation | Game Boy Color (original Game Boy comes with it) |
| Controls | 8 buttons: D-pad (Up/Down/Left/Right), A, B, Start, Select |
| Audio | Small speaker + SOIC-8 amplifier chip, soldered by the owner |
| Power | USB-C only (through the dev board), no battery |
| Extras | Power slide switch, one status LED |
| Layout | Classic portrait Game Boy layout |
| Case | None, the bare PCB is the body |
| Board look | Standard green soldermask |
| Soldering | Through-hole plus a few big SMD parts (0805, SOIC-8). No fine-pitch, no BGA. |
| Fab | JLCPCB bare boards (no assembly); owner solders everything |
| Budget | About $25-40 total, shipped to Germany |
| Owned already | ESP32-S3 N16R8, soldering iron, multimeter |
| Owner experience | Perfboard with through-hole only. Never ordered or soldered a PCB. |

## 3. Hardware overview

```
USB-C ──► [ESP32-S3 N16R8 dev board on sockets]
              │ SPI ──► 2.8" ILI9341 LCD (+ microSD on the same module)
              │ GPIO ◄── 8 buttons (+ optional L/R pads later)
              │ PWM/I2S ► amplifier (SOIC-8) ──► speaker
              └ GPIO ──► status LED;  power switch in the 5 V feed to peripherals
```

### 3.1 ESP32-S3 pin planning (VERIFY against the datasheet and the dev board schematic)
- N16R8 uses octal flash/PSRAM, which takes **GPIO26-GPIO37**. Do not use them.
- Strapping pins (**GPIO0, 3, 45, 46**) must not be pulled by buttons or the LED at boot.
- **GPIO19/20** are native USB; **GPIO43/44** are UART0. Keep them free.
- Buttons: 8 inputs on free GPIOs with internal pull-ups, one side to GND, small RC or
  firmware debouncing. Choose pins that avoid all of the above.
- The LCD module's SD slot shares the SPI wiring; it needs its own chip-select.

### 3.2 Screen
- A typical 2.8" ILI9341 module has about 14 pins: VCC, GND, CS, RESET, DC, MOSI, SCK, LED
  (backlight), MISO, plus touch and SD pins. **VERIFY the exact module before drawing the
  footprint.** Pin order and voltage (3.3 V or 5 V supply) differ between sellers.
- 320x240 at 16 bit over SPI needs roughly 40 MHz or faster for smooth 60 fps; the Game Boy
  image is 160x144 and can be scaled.

### 3.3 Audio (design decision for the schematic phase)
- The ESP32-S3 has **no DAC**. Easiest hand-solderable path: PWM or I2S output through an
  RC low-pass filter into a SOIC-8 analog amplifier (for example PAM8302A, **VERIFY**
  availability and footprint).
- Higher quality would use an I2S amplifier chip, but common ones are tiny QFN parts. If
  sound quality disappoints, fall back to an I2S amplifier module on headers.

### 3.4 Power
- 5 V comes from the dev board's USB-C port. A slide switch in the 5 V line to the screen
  and amplifier turns the peripherals off. It does **not** cut power to the ESP32 itself, so
  document that clearly on the silkscreen, or decide to route the switch differently.
- Add a bulk capacitor near the amplifier and the LCD supply.

## 4. Software

- Emulator candidates (VERIFY current state before relying on them): CrankBoy and Peanut-GB
  derived emulators run original Game Boy at about full speed on an ESP32-S3 (see sources in
  section 8). **Game Boy Color on ESP32-S3 is not confirmed.** Treat as a risk.
- Possible speed-up: faster octal PSRAM clock (120 MHz) is an experimental ESP-IDF option.
  VERIFY in the current ESP-IDF docs before using. The CPU's official maximum is 240 MHz, so
  no overclocking.
- ROMs: use homebrew or games you own.

## 5. Bill of materials (rough, unverified, USD)

| Item | Est. |
|---|---|
| 2.8" 240x320 SPI LCD module with SD slot | 6-10 |
| PCB, 5 pieces (2-layer) | 2-5 |
| Shipping to Germany (PCBs and/or parts) | 8-20 |
| 8 tactile buttons + power slide switch | 2-3 |
| Amplifier chip + speaker | 2-4 |
| Female sockets for the ESP32 + male headers for the LCD | 2-4 |
| Resistors, capacitors, LED (0805) | 1-3 |
| **Total** | **about 25-50** |

The top of this range is above the stated $25-40 budget. Shipping is the swing factor, and
consolidating into as few parcels as possible matters more than any single part.

## 6. Risks and open questions

1. **ESP32-S3 dev board:** confirmed as the **diymore "ESP32 S3 DevKitC 1 N16R8"** (Amazon.de
   listing, male pins already soldered), a DevKitC-1 clone with two 22-pin headers and two
   USB-C ports. Its labels match the standard DevKitC-1 layout (VERIFY against the seller's pin
   diagram, clones can differ). GPIO35-37 are broken out on the board but used internally by the
   octal PSRAM, so they must stay unconnected. **Header row spacing is 22.86 mm (0.9 in)**,
   read from the hole coordinates in Espressif's own DXF (columns at x = 1.270 and 24.130 mm,
   2.540 mm pitch, 22 holes per row; the 25.4 mm figure is the board WIDTH, holes sit 1.27 mm in
   from each edge). An earlier version of this spec said 25.4 mm, taken from a KiCad forum post;
   that was wrong and is corrected here. This is Espressif's official board: the owner's diymore
   clone may differ, so the 1:1 paper-template check in section 7 (step 4) is still required.
2. **Exact LCD module listing** to buy. Pinout and supply voltage must match the footprint. (Open.)
3. **Game Boy Color speed on the S3** is unconfirmed. Mitigation: test the emulator on the
   owner's board with the screen on a breadboard *before* ordering the PCB.
4. **Budget** may be exceeded by shipping. Mitigation: get a real JLCPCB quote from generated
   files before ordering.
5. **First-ever PCB.** Mitigation: DRC, a 3D check in KiCad, a printed 1:1 paper copy to test
   part fit, and an independent netlist review before ordering.
6. Audio quality with PWM and a small speaker will be modest.

## 7. Plan (nothing below has been started)

1. Owner confirms the dev board model and picks an LCD module listing.
2. Smoke-test the emulator on the real S3 + LCD on a breadboard (no PCB yet).
3. Generate the schematic with SKiDl, run ERC.
4. Place parts, route with Freerouting, run DRC against JLCPCB rules, review 3D view and a 1:1 print.
5. Generate fabrication files and a real JLCPCB quote; owner decides whether to order.
6. After the board arrives: solder in the order through-hole last, SMD first, test power with the
   multimeter before plugging in the ESP32.

## 8. Sources (searched during the planning conversation)

- 44vba, GBA emulator for ESP32-S3: https://github.com/44670/44vba
- PaperBoy S3, Game Boy at 60 fps on ESP32-S3: https://www.cnx-software.com/2026/07/02/paperboy-game-boy-emulator-works-at-60-fps-on-esp32-s3-e-ink-devkit/
- ESP32-S3 vs P4 guide: https://www.elecrow.com/blog/esp32-s31-vs-s3-vs-p4-complete-2026-selection-guide.html
- Teensy Game Boy emulator: https://www.pjrc.com/game-boy-emulator
- DevKitC-1 official dimensions (PDF and DXF, hole coordinates give 22.86 mm row spacing): https://dl.espressif.com/dl/schematics/esp_idf/DXF_ESP32-S3-DevKitC-1_V1.1_20220429.pdf and .dxf
- (Incorrect) KiCad forum claim of 25.4 mm row spacing, superseded by the DXF above: https://forum.kicad.info/t/pin-distance-in-esp32-s3-devkitc-1/71001
- ESP32-S3-DevKitC-1 user guide: https://docs.espressif.com/projects/esp-idf/en/v5.2.3/esp32s3/hw-reference/esp32s3/user-guide-devkitc-1.html

## 9. Schematic status (v0.1, step 3 of the plan)

Files: `gbc-handheld/schematic.py` (SKiDl source), `gbc-handheld/build/gbc_handheld.net`
(generated KiCad netlist), `gbc-handheld/build/pinmap.md` (generated GPIO table),
`gbc-handheld/verify_netlist.py` (independent checker).

Reproduce: `source /etc/profile.d/eda.sh && python3 gbc-handheld/schematic.py &&
python3 -I gbc-handheld/verify_netlist.py gbc-handheld/build/gbc_handheld.net`

What was checked
- SKiDl ERC: 0 errors, 0 warnings.
- Independent netlist check: all checks pass on the real netlist (power rails, amp pinout, LCD
  header, SD header, switch, LED, 8 buttons, every GPIO header position, forbidden pins left
  unconnected). The checker was also run on two deliberately broken copies and failed them.
- Netlist converted to a real KiCad PCB (`kinet2pcb`): all 29 footprints load, 32 nets.

Facts verified from primary sources while building it
- ESP32-S3-DevKitC-1 J1/J3 pin order: Espressif user guide v1.1 tables.
- PAM8302A SOP-8 pinout read from the Diodes datasheet drawing: 1 /SD, 2 NC, 3 IN+, 4 IN-,
  5 VO+, 6 VDD, 7 GND, 8 VO-. Gain A = 20 log[2 (RF/RI)], RI min 10k internal, RF 80k.
- **Buy `PAM8302AADCR` (SOP-8).** `PAM8302AASCR` is the MSOP-8 package, too small to hand-solder.
- KiCad has no PAM8302A symbol (only the different PAM8301), so the script defines its own.
- MSP2807 14-pin header: 1 VCC, 2 GND, 3 CS, 4 RESET, 5 DC, 6 SDI(MOSI), 7 SCK, 8 LED,
  9 SDO(MISO), 10-14 touch. The datasheet lists **no SD pins**.

Design values
- GPIO: buttons 1,2,4,5,6,7,15,16; LCD 8-13; status LED 14; audio PWM 17; SD CS 21; 18 reserved.
  The script refuses (assert) any signal on GPIO 0,3,19,20,26-38,43-48.
- Audio: PWM -> 1k + 10nF low-pass (~16 kHz) -> 220nF + 22k per input -> PAM8302A. Gain is about
  14 dB (2 x 80k / (10k + 22k)); firmware must keep the PWM amplitude low to avoid clipping.
  Supply decoupling 1uF + 10uF per the datasheet. /SD pulled high (always on).
- Power switch: SPDT, common = USB 5 V, throw = switched 5 V for the LCD and amp only.
- LCD backlight pin from 3V3 through a 0R resistor (R_BL), per the MSP2807 datasheet note.

Open items (must be resolved before layout is frozen)
1. **SD pads (resolved in design, verify on the real module).** The module's SD interface is
   four bare round **solder pads** on its back (datasheet photo, DIANN listing photo), not a
   header, and they cannot plug into a socket. `J_SD` is now a 4-pin header on our PCB; the
   owner solders four short wires from the module's SD pads to it, in the order SD_CS, SD_MOSI,
   SD_MISO, SD_SCK (silkscreen read bottom to top). Confirm the pad labels on the real module.
   Fallback if wiring is unwanted: drop the module slot and use ROMs from the 16 MB flash.
2. **Backlight and power switch.** With the switch off and USB plugged in, 3V3 still reaches the
   LCD LED pin through R_BL; the module may glow faintly or back-feed. Check the module's backlight
   circuit; fix by removing R_BL or switching 3V3 too.
3. **Speaker choice** (8 ohm, size) and whether to add a ferrite bead / bulk cap for EMI.
4. **Footprints are 2.54 mm sockets** sized for the DevKitC-1 at 25.4 mm row spacing; confirm with
   the 1:1 paper printout before ordering.
5. **Game Boy Color speed** on the S3 is still unconfirmed (section 4).

## 10. LCD module: chosen listing and mechanical data

Listing the owner found: **DIANN 2.8" ILI9341 SPI TFT 240x320, "No Touch Function" variant**
(Amazon.com, $9.99 plus about $13.94 shipping and import charges to Germany, about $24 total
for one module at the time of the screenshot). The back-of-board photo shows silkscreen
matching the MSP2807 pin order, "V1.1", and the four SD pads. Not yet bought.

**Cost warning:** shipping and import charges are more than the module. The $25-40 project
budget would be mostly spent on this one part. Look for the same module sold within the EU
(for example amazon.de or a German electronics shop) before buying. The pinout and drawing
below are the MSP2807 reference design; a clone may differ slightly, so measure the real
module before ordering PCBs.

Mechanical data from the manufacturer's outline drawing (`MSP2807_Size.pdf` on lcdwiki.com,
revision V1.0, PCB marked V1.2), unmarked tolerance +/-0.2 mm. Module held portrait, front view:

| Item | Value |
|---|---|
| PCB size | 50.00 x 86.00 mm, 1.60 mm thick |
| Glass (LCD) | 50.00 x 69.20 mm; viewable 45.20 x 59.45; active 43.20 x 57.60 |
| Mounting holes | 4x, 3.20 mm drill (4.70 mm pad), spacing 44.00 x 76.08 mm, 3.00 mm from the sides, top pair 3.00 mm from the top edge |
| 14-pin header | on the bottom short edge, centred on the 50 mm width, 13 x 2.54 = 33.02 mm long, 8.49 mm from each side, pin row 2.00 mm up from the bottom edge |
| Header pin order | back view: pin 14 (T_IRQ) left ... pin 1 (VCC) right; so pin 1 is on the left in the front view |
| Heights | total thickness excluding header 5.60 mm; header height 11.17 mm; total including header 12.78 mm |
| SD slot | on the back, right of centre; opens toward one long edge. Keep tall parts out from under it |
| SD pads | 4 round pads (labels SD_SCK, SD_MISO, SD_MOSI, SD_CS), not on the 14-pin header |

Consequences for the board
- A 1x14 female socket at 2.54 mm pitch matches the header (33.02 mm span confirmed).
- Add four **M3 standoffs and screws** (hole positions above) to carry the module; standoff
  length must match the socket height (about 8.5 mm for a standard female header; VERIFY
  against the pins' protrusion, 5.84 mm in the drawing).
- If the module is mounted landscape (86 x 50, rotated 90 degrees clockwise), holes land at
  x = 6.92 and 83.00 mm, y = 3.00 and 47.00 mm from the module's top-left corner, and the
  header runs vertically along the left edge with pin 1 at the top.
- Add four short wires (module SD pads to `J_SD`), and keep a clear zone under the SD slot.

## 11. ESP32-S3-DevKitC-1 mechanical data (Espressif DXF, board V1.1)

| Item | Value |
|---|---|
| Board size | 25.40 x 62.74 mm |
| Header rows | 2 x 22 holes, 2.54 mm pitch, **22.86 mm between the rows**, 1.27 mm in from each long edge |
| Pin 1 end (antenna end) | first hole 1.44 mm from the board end |
| Pin 22 end (USB end) | last hole **7.96 mm** from the board end (USB connectors sit there) |
| Hole span | 21 x 2.54 = 53.34 mm |
| Row/pin mapping | antenna end up, USB down, component side up: J1 is the LEFT row, J3 the RIGHT row |

Layout consequence: to keep the board within JLCPCB's cheapest size tier (100 x 100 mm,
USD 2 for 5 two-layer boards; larger boards cost more, exact price from their calculator),
the ESP32 board is mounted on the **underside** of the PCB below the LCD, with its USB end
overhanging a board edge.

## 12. Hand-solderability requirement (beginner)

The owner has only soldered through-hole parts on perfboard and has never soldered SMD or
ordered a PCB. **The board must be hand-solderable by a beginner.** Decision: no SMD parts.
All components are through-hole or plug-in (sockets, headers, tactile switches, slide switch,
axial or radial resistors and capacitors, a 3 mm LED, and a plug-in amplifier module). The
SMD PAM8302A design from schematic v0.1 (sections 3.3 and 9) is **superseded** by v0.2, see
section 13.

## 13. Schematic v0.2 (all through-hole) — current design

Decision (owner): audio amplifier = **MAX98357A I2S digital amplifier module** on a 7-pin socket.
Everything else is through-hole or plug-in. 19 parts, no SMD footprints.

| Ref | Part | Footprint |
|---|---|---|
| J1, J3 | ESP32-S3-DevKitC-1 header sockets, 2 x 1x22 female | 2.54 mm socket, rows 22.86 mm apart |
| J_LCD | LCD module header socket, 1x14 female | 2.54 mm socket |
| J_SD | 4-pin header, 4 wires to the module's SD pads | 2.54 mm pin header |
| J_AMP | MAX98357A module socket, 1x7 female | 2.54 mm socket |
| JP_BL | backlight jumper, 2-pin header + shunt | 2.54 mm pin header |
| SW1-SW8 | 6 mm tactile buttons (D-pad, A, B, Start, Select) | THT 6 mm |
| SW_PWR | SPDT slide switch (C&K OS102011MS2Q type) | THT |
| R_LED | 1k, 1/4 W axial | THT |
| D_STATUS | 3 mm LED | THT |
| C_BULK | 100 uF electrolytic, >=10 V, radial 5 mm | THT, polarised |
| C_HF | 100 nF ceramic, disc | THT |

Also needed, not on the PCB: 4x M3 standoffs + screws (LCD), speaker (4-8 ohm, 3 W max) wired to
the amp module's own terminals, 4 short wires for the SD pads, one jumper shunt.

Audio: I2S on GPIO17 (BCLK), GPIO18 (LRC), GPIO40 (DIN), GPIO41 (SD / mute). The module's own
SD pull-up gives a mono mix; the ESP32 can pull SD low to mute. GAIN is left open (9 dB).
Facts from the Adafruit guide: SD below 0.16 V = shutdown, 0.16-0.77 V = (L+R)/2, 0.77-1.4 V
= right only, above 1.4 V = left only; GAIN open = 9 dB; VIN 2.7-5.5 V. Driving SD high from a
3.3 V GPIO selects LEFT only, so use open-drain/high-Z for "on" in firmware, low for mute.

Checks on v0.2: SKiDl ERC clean; `verify_netlist.py` 101 checks pass (and fails a deliberately
broken copy); netlist converts to a PCB with 19 footprints, none SMD.

Open items before ordering
1. **MAX98357A module pin order.** The header order `LRC BCLK DIN GAIN SD GND VIN` is assumed
   from the Adafruit breakout; clones may differ or reverse it. Pick the listing, read its
   silkscreen photo, and set `AMP_ORDER` in `schematic.py` (one line). Module is about 19 x 18 mm.
2. LCD module: listing with shipping inside the EU; confirm SD pad labels and the 14-pin header
   dimensions on the real module (1:1 paper template before ordering PCBs).
3. DevKitC-1 clone dimensions vs Espressif's drawing (section 11): paper template check.
4. Speaker size and mounting (not on the PCB).
5. Game Boy Color speed on the S3 (section 4), unconfirmed.

## 14. Board layout v0.2 (placed, routed, checked)

Files: `gbc-handheld/build_board.py` (place, autoroute, fix thermals), `build/gbc_handheld.kicad_pcb`
(the routed board), `build/gbc_handheld_gerbers.zip` (JLCPCB upload), `build/template.pdf`
(1:1 paper template), `build/BOM.md` / `BOM.csv` (shopping list), `ASSEMBLY.md` (soldering guide),
`build/r_top.png`, `build/r_bottom.png` (renders), `build/drc.rpt`.

Design
- 2 layers, 90 x 98 mm, 3 mm corner radius, ground fill on both layers.
- Top side: everything except the ESP32 sockets. LCD module sits above the board on four M3
  standoffs, landscape (the portrait module rotated 90 degrees clockwise), header socket at the left.
- Underside: ESP32-S3-DevKitC-1 on two 1x22 sockets, antenna end left, USB end overhanging the
  right edge by 4.3 mm. Rows 22.86 mm apart, J1 above J3 as seen from the top.
- Tracks 0.3 mm, power nets (+5V_USB, +5V_SW, +3V3) 0.6 mm. Vias 0.8/0.4 mm. Clearance 0.25 mm.
- Cost tier: 90 x 98 mm stays inside JLCPCB's cheapest size tier (100 x 100 mm). Check the live quote.

Checks run (all software; nothing tested on hardware)
1. Geometry script: socket rows 53.34 mm long and 22.86 mm apart, LCD header 33.02 mm, mounting
   holes 76.08 x 44.00 mm apart, USB end overhang >= 3 mm, all parts inside the outline margin.
2. KiCad DRC: 0 violations, 0 unconnected items (starved thermal-relief warnings on ground pads
   are fixed automatically by solid-connecting those pads and refilling).
3. `verify_board.py`: every pad's net equals the netlist, nothing unrouted, tracks >= 0.2 mm, power
   tracks >= 0.5 mm, smallest hole >= 0.3 mm, 2 layers, no SMD, only J1/J3 on the underside. A
   deliberately wrong net in a copy of the board is reported as a failure.
4. Gerber re-read with gerbonara (separate from KiCad): outline 90.10 x 98.10 mm, 125 plated
   holes = 120 pad holes + 5 vias, no unplated holes.
5. Paper template measured at true scale (90.1 x 98.1 mm).

Not verified
- Fit of the physical parts: the owner's DevKitC-1 clone, the chosen LCD module and the MAX98357A
  module against the 1:1 template (`ASSEMBLY.md`, "Before you order").
- MAX98357A header pin order, SD pad labels, backlight behaviour with the switch off (section 13).
- Whether the amplifier module's body and speaker terminals clear the nearby buttons and the LCD.
- Firmware: Game Boy Color speed on the S3 is unconfirmed.
- Real-world: no board has been built.

## 15. Full audit (v0.3): what was checked, what was wrong, what changed

Requested: "check it a lot, with more images and tools, so it is 100% working: no cutting, no
wrong tracks, no wrong footprints, everything fits". Everything below runs with one command,
`gbc-handheld/check_all.sh`, which exits non-zero if anything fails. Each checker was also run
on a deliberately broken input to prove it can fail (netlist with a renamed net, board with a
wrong pad net, ESP32 mounted unflipped, drill file shifted 0.2 mm): all were rejected.

### Problems found and fixed

| # | Problem | How it was found | Fix |
|---|---|---|---|
| 1 | LCD standoffs specified as 8.5 mm. The LCD's pin header has a 2.54 mm plastic spacer, so the module sits 8.5 + 2.54 = 11.04 mm above the board | MSP2807 drawing side view: 8.38 - 5.84 = 2.54 | **11 mm standoffs**; BOM requires 8.5 mm tall sockets |
| 2 | Slide switch C&K OS102011MS2Q is rated **100 mA**; it fed the LCD and the 3 W amp (~0.6 A peaks) | rating looked up (Digi-Key) | switch is now the **SS-12D00G3 (0.5 A)** with a project footprint, and it only feeds the LCD (~0.1 A); the amp runs from USB 5 V and is muted by GPIO41 |
| 3 | No way for firmware to know the switch position | design review | 15k/22k divider to GPIO39 (PWR_SENSE): 2.82-3.12 V across USB 4.75-5.25 V |
| 4 | LCD SDO (MISO) shared with the SD card; some ILI9341 boards do not release it | design review | LCD pin 9 left open (datasheet: allowed); SPI_MISO connects only ESP32 and SD |
| 5 | Backlight jumper fed 3.3 V even with the LCD switched off | design review | backlight from GPIO42 through 1k (dimmable; safe even if the pin drives LEDs directly) |
| 6 | Electrolytic caps: + and - pads only **0.40 mm** apart (KiCad 2.0 mm-pitch footprint), a short across 5 V if bridged | `check_3d.py` pad-gap check | 2.5 mm-pitch footprint: 0.9 mm gap |
| 7 | Freerouting treated the ground fill as a perfect plane and left **C_HF's ground pad floating** in a cut-off copper island | KiCad DRC after the fill (Freerouting itself said "0 unrouted") | ground is routed as real tracks; the fill is added afterwards; every route attempt is accepted only if KiCad reports 0 unconnected |
| 8 | Silkscreen labels 0.7 mm tall (below the 0.8 mm fab minimum) | KiCad DRC | all silkscreen text >= 1.0 mm |
| 9 | Project footprint library and net-class rules not saved with the board, so KiCad's DRC could not enforce the 0.6 mm power tracks | KiCad DRC (`lib_footprint_issues`) | board saved with its `.kicad_pro` and a project `fp-lib-table` |
| 10 | Shopping list said "widerstand 1k" for the 15k and 22k resistors | reading the generated BOM | search text uses each part's value |
| 11 | Amp module plugged in pointing up would hit the LCD | `check_3d.py` | silkscreen "BODY POINTS DOWN" + assembly step |

Two alarms were the checkers, not the board: the Gerber checker could not size KiCad's
rounded-rectangle pad apertures (fixed: measure real outlines), and gerbonara's SVG renderer
draws one board corner wrong (the Gerber arcs were checked by hand and rendered correctly with
gerbv; gerbv now makes the Gerber pictures).

### Checks and results on the final board

| Check | Tool | Result |
|---|---|---|
| Schematic ERC | SKiDl | 0 errors, 0 warnings |
| Netlist connections (amp, LCD, SD, switch, divider, LED, 8 buttons, every ESP32 header pin, forbidden pins) | `verify_netlist.py` | all pass |
| Design rules incl. power-track net class, silkscreen, courtyards, thermal reliefs | KiCad DRC | 0 violations, 0 unconnected |
| Every pad's net equals the netlist; track widths; holes; size; layers; no SMD | `verify_board.py` | all pass |
| Real ESP32-S3-DevKitC-1 plugs in: all 44 pins land on a pad with the right signal | `check_plugfit.py` + Espressif DXF | 44/44 |
| 3D: heights under the LCD (<= 8.84 mm) and amp; standoffs; screw heads; ESP32 below; SD card path; buttons reachable; pad gaps >= 0.6 mm; annular rings | `check_3d.py` | all pass |
| Manufacturing files: outline 90 x 98 mm, every drill hit on copper both sides, rings >= 0.15 mm, mask openings, hole count | `check_gerbers.py` (gerbonara) + gerbv render | all pass |

### Pictures (in `gbc-handheld/build/`)

`3d_top.png`, `3d_bottom.png`, `3d_angled.png` (KiCad 3D renders with part models),
`r_top.png`, `r_bottom.png` (layout incl. module outlines on the Fab layers),
`gerber_top.png`, `gerber_bottom.png` (the manufacturing files drawn by gerbv),
`fit_top.png` (every part coloured by height, modules and standoffs outlined),
`fit_sections.png` (side cuts through the stack-up), `zoom_esp_top.png`, `zoom_esp_bot.png`
(close-ups of the densest routing), `template.pdf` (1:1 paper template).

### Still not verifiable from here

- Your clone boards and modules against the drawings (paper template step in `ASSEMBLY.md`).
- MAX98357A module pin order and size on the listing you buy; LCD SD pad labels.
- Heights of the parts you actually buy (the table in `check_3d.py` lists every assumption).
- Real-world behaviour: no board has been built; firmware is not written yet.
