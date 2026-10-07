# Assembly guide, board v0.3 (through-hole only)

Everything on this board is through-hole or plug-in. Nothing needs hot air or fine-pitch skill.
You need: soldering iron (about 320-350 C), thin solder, side cutters, multimeter, tape.
Parts: `build/BOM.md`. Pictures of the finished board: `build/3d_top.png`, `build/3d_angled.png`.

## Before you order the PCB (free, about 10 minutes)

1. **Print `build/template.pdf` at 100% / "Actual size"** (not "fit to page"). Check with a ruler
   that the drawn board is 90.1 x 98.1 mm.
2. **ESP32 board:** from underneath, push its 44 pins up through the two rows of holes
   (antenna end left, USB end sticking out on the right). All pins must go in without bending.
   The software check `check_plugfit.py` already proves Espressif's official board fits pin for
   pin; this paper test proves *your clone* matches Espressif's drawing.
3. **LCD module:** push its 14 header pins through the 14-hole column on the left. The module's
   four corner holes must sit over the four large round holes.
4. **Amplifier module:** hold it over the 7-hole row with its body pointing **down** (towards the
   buttons). Its pin names must read `LRC BCLK DIN GAIN SD GND VIN` from left to right, exactly as
   printed on the board. If the order is reversed, stop and tell me (one line to change).
5. Upload `build/gbc_handheld_gerbers.zip` to JLCPCB: 2 layers, quantity 5, leave assembly OFF.
   Check the live price before paying.

## Soldering order (low parts first, tall parts last)

Heat pad and lead together for 2-3 seconds, feed solder, remove solder then iron. Good joints
are small shiny cones. Trim leads after soldering.

1. **ESP32 sockets J1 and J3 go on the BOTTOM face; solder them from the TOP.** Trick: push both
   female sockets onto your ESP32 board's pins, then push the whole thing up through the PCB from
   below so the board holds the sockets straight. Tape it, flip the PCB, solder all 44 pins, then
   pull the ESP32 board out.
2. **Resistors:** R_BL (1k), R_LED (1k), R_S1 (**15k**), R_S2 (**22k**). Check the values with the
   multimeter before soldering: R_S1 and R_S2 must not be swapped.
3. **C_HF (100 nF):** push it all the way down; it sits under the LCD and must stay below 8 mm.
4. **LED D_STATUS:** the flat side of the LED and the square pad are the minus side.
5. **Pin header J_SD** (short pin ends into the board).
6. **Sockets J_LCD (14) and J_AMP (7):** use the modules as a jig (socket on the module's pins,
   set onto the board, solder, remove the module).
7. **8 buttons** and the **slide switch SW_PWR**.
8. **C_BULK and C_AMP (100 uF electrolytics), last: polarity matters.** Long leg = plus = the
   square pad next to the `+` mark. Bend the legs slightly outwards; the holes are 2.5 mm apart.

## Before power-on (multimeter, nothing plugged in)

1. Continuity mode: probe **J_LCD pin 1 (VCC, marked) and pin 2 (GND)** with the slide switch in
   both positions, then **J_AMP pin VIN and GND**. A short chirp that stops is the capacitor
   charging (normal). A **continuous beep is a short**: find the bridge before going on.
2. Look at J1, J3, J_LCD and J_AMP from underneath for bridges between neighbouring pins.
3. Find which way is ON: with the slide switch in one position, continuity between the switch's
   middle pin and J_LCD pin 1 means that position is ON. Remember it.
4. Plug in the ESP32 board (from below), then USB. Voltage mode: 5 V at J_AMP VIN always;
   5 V at J_LCD pin 1 only with the switch ON; about 3 V (to GND) on the two inner pads where
   R_S1 and R_S2 face each other (that is PWR_SENSE) with the switch ON, 0 V with it OFF.
   If any is wrong, unplug and re-check.

## Then

- Solder four short wires from the LCD module's SD pads to J_SD: SD_CS -> CS, SD_MOSI -> MOSI,
  SD_MISO -> MISO, SD_SCK -> SCK (names are printed on the board under J_SD).
- Mount the LCD on four **11 mm** M3 standoffs (screws from below), then plug it into J_LCD.
  The 11 mm matches the 8.5 mm socket plus the 2.54 mm spacer on the LCD's pins.
- Plug the amplifier into J_AMP with its body pointing **down**; wire the speaker to it.
- Use only one of the ESP32 board's two USB ports at a time.

## What the switch does (firmware note)

The slide switch powers the LCD only. The ESP32 reads it on GPIO39 (PWR_SENSE): when it reads
low, firmware should mute the amplifier (GPIO41 low), turn the backlight off (GPIO42 low), stop
driving the LCD pins and go to sleep. The amplifier is powered straight from USB because it can
draw more current than a mini slide switch is rated for.

## Known limits

- Nothing here has been built or tested on real hardware. The checks are software checks; see
  `check_all.sh` and spec section 15.
- GPIO pins used: `build/pinmap.md`. Never use GPIO 0, 3, 19, 20, 26-38, 43-48 on this board.
