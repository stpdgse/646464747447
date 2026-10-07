# Assembly guide (for a first PCB, through-hole only)

Everything on this board is through-hole or plug-in. Nothing needs hot air or fine-pitch skill.
You need: soldering iron (about 320-350 C), thin solder, flux (optional), side cutters,
multimeter, tape, a few hours. Parts: see `build/BOM.md`.

## Before you order the PCB (free, takes 10 minutes)

1. **Print `build/template.pdf` at 100% / "Actual size"** (not "fit to page"). The drawn board
   must measure 90.1 x 98.1 mm with a ruler.
2. **ESP32 board:** from underneath, push your board's 44 pins up through the paper holes
   (both rows). All pins must go in without bending, USB end to the right, antenna end left.
   If they do not line up, stop: your clone differs from Espressif's drawing (rows 22.86 mm
   apart) and the PCB must change. Tell me and I will adjust one number.
3. **LCD module:** from above, push its 14 header pins through the 14-hole column at the left
   edge and check the four corner holes of the module line up with the four large holes.
4. **Amplifier module:** hold it over the 7-hole row and check the silkscreen order on the module
   (it must read `LRC BCLK DIN GAIN SD GND VIN` left to right with the front facing you).
5. Only then order the PCBs: upload `build/gbc_handheld_gerbers.zip` to JLCPCB, 2 layers, green,
   quantity 5, HASL lead-free, leave "assembly" OFF. Check the live price first.

## Soldering order (low parts first, tall parts last)

Solder each joint from the side opposite the part: heat pad and lead together for 2-3 seconds,
feed solder, remove solder then iron. Joints should look like small shiny cones.

1. **ESP32 sockets J1 and J3 (underside!).** These go on the BOTTOM face, solder from the TOP.
   Easiest: push the two female sockets onto your ESP32 board's pins, then push the whole thing
   up through the PCB from below so the board acts as a jig. Tape it down, flip the PCB over,
   solder all 44 pins from the top. Then pull the ESP32 board out and trim nothing.
2. **R_LED (1k), then C_HF (100 nF).** Parts on top, solder underneath, trim the leads.
3. **LED (D_STATUS):** the flat side of the LED and the square pad are the cathode (minus).
4. **Pin headers JP_BL and J_SD.** Short pin ends go through the board, long ends up.
5. **Sockets J_LCD (14 pins) and J_AMP (7 pins).** Use the real modules as a jig: push the
   socket onto the module's pins, then set the pair onto the PCB, solder from underneath, then
   remove the module. Check both are straight and flat.
6. **Tactile buttons SW1-SW8** (8 pieces). They click into the board; solder all four legs.
7. **Slide switch SW_PWR.** Solder the three pins and the two side tabs.
8. **C_BULK (100 uF electrolytic): mind the polarity.** The long leg is plus; the stripe on the
   can is minus. The board is marked `+` on the square pad side. Solder last, it is tallest.

## Before you power it on (multimeter, no USB yet)

1. Continuity (beep) mode, slide switch ON, nothing plugged in: measure between **J_LCD pin 1
   (the +5 V pin, top of the socket) and J_LCD pin 2 (GND)**. A short chirp that stops is just
   the capacitor charging and is normal. A **continuous beep means a short**: find the solder
   bridge and fix it before going on. Repeat with the slide switch OFF.
2. Look at the pins of J_LCD and J1 / J3 from underneath for tiny bridges between neighbours.
3. Plug in the ESP32 board (USB end overhangs the right edge), then connect USB. With a
   multimeter in voltage mode, 5 V should appear between J_LCD pin 1 and pin 2 with the slide
   switch ON and 0 V with it OFF. If either is wrong, unplug and re-check before adding modules.

## Then

- Solder four short wires from the LCD module's four SD pads to J_SD (order on the module's
  silkscreen, bottom to top: SD_CS, SD_MOSI, SD_MISO, SD_SCK; they go to J_SD pins 1-4).
- Fit the jumper shunt on JP_BL (powers the backlight).
- Mount the LCD on four M3 standoffs, plug into J_LCD. Plug the amplifier module into J_AMP and
  wire the speaker to the module's own terminals.
- Do not plug the USB cable into BOTH of the ESP32 board's USB ports at once.

## Known limits of this first board

- The power switch cuts the LCD and amplifier, **not** the ESP32 itself.
- GPIO pins on this board are listed in `build/pinmap.md`; do not use GPIO 0, 3, 19, 20, 26-38,
  43-48 (see the spec for why).
- I have not tested any of this on real hardware. The checks I ran are software checks
  (schematic ERC, netlist vs. board comparison, KiCad DRC, Gerber and drill re-reading).
