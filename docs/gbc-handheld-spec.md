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
   octal PSRAM, so they must stay unconnected. Header row spacing is **25.4 mm (1 in)** per a KiCad
   forum thread (see section 8). **Trap:** the official KiCad library footprint for this board
   reportedly uses 22.86 mm, which is wrong; do not use it unchecked. The 1:1 paper printout
   check in section 7 (step 4) covers this against the owner's real board.
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
- DevKitC-1 header row spacing (25.4 mm vs library 22.86 mm): https://forum.kicad.info/t/pin-distance-in-esp32-s3-devkitc-1/71001
- ESP32-S3-DevKitC-1 user guide: https://docs.espressif.com/projects/esp-idf/en/v5.2.3/esp32s3/hw-reference/esp32s3/user-guide-devkitc-1.html
