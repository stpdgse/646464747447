# Controls

Piloting starts when you right-click a placed drone (the drone is placed with the **FPV Drone** item). Your body stays
in the world while the camera moves to the drone. Sneak + right-click picks a drone back up.

All three input devices feed the same pipeline (`input/`), so they behave identically: deadzone → rates/expo (in the
flight sim) → server. Touch/on-screen sticks come in a later phase through the same `InputSource` interface.

## Keyboard (default; all rebindable under *Controls → Kamikaze Skies - Flight*)
| Action | Key |
|---|---|
| Pitch forward (nose down) / back | `W` / `S` |
| Roll left / right | `A` / `D` |
| Yaw left / right | `Q` / `E` |
| Throttle up / down | `Space` / `Left Shift` |
| Arm / disarm | `R` |
| Cycle assist mode (Angle → Mixed → Acro) | `F` |
| Take goggles off | `G` |

- Keys are binary, so sticks ramp toward full deflection while held and spring back on release.
- Throttle behaves like a real non-spring stick: it stays where you leave it.
- Like a real radio, **arming is refused unless the throttle is low**; disarming always works.
- These keys only act while piloting (separate key-conflict context), so they do not clash with vanilla movement.

## Gamepad (Mode 2 radio layout, via SDL3)
| Action | Input |
|---|---|
| Throttle / yaw | Left stick (up-down / left-right) |
| Pitch / roll | Right stick (up-down / left-right) |
| Arm / disarm | `A` (south) |
| Cycle assist mode | `Y` (north) |
| Take goggles off | `B` (east) |

Throttle is *direct* by default (stick up = 100%); an *incremental* mode (stick moves the throttle, which then stays)
exists in code and will get a setting. Gamepad becomes active when you touch a stick; keyboard takes over again when
you press a key. **Not yet verified with real hardware** (no device in the cloud test environment); the mapping and
deadzone logic is unit-tested and the SDL polling initialises cleanly.

## Assist modes
| Mode | Behaviour |
|---|---|
| ANGLE | Self-leveling. Sticks set a target tilt (up to 55°). Easiest. |
| MIXED | 50% blend of angle and rate control. Forgiving but lively. |
| ACRO | Pure rate control; the drone holds whatever attitude you leave it in. Flips and rolls. |

## On-screen display
Armed state, assist mode, input device, motor power, speed (km/h) and altitude above ground. Warnings: *arming refused*.
